//! Process management syscalls
use core::mem::size_of;

use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{translated_byte_buffer, translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next, free_memory,
        new_memory, suspend_current_and_run_next,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let buffers =
        translated_byte_buffer(current_user_token(), ts as *const u8, size_of::<TimeVal>());

    let bytes = unsafe {
        core::slice::from_raw_parts(
            &time_val as *const TimeVal as *const u8,
            size_of::<TimeVal>(),
        )
    };
    let mut current = 0;
    for buffer in buffers {
        let len = buffer.len().min(bytes.len() - current);
        buffer[..len].copy_from_slice(&bytes[current..current + len]);
        current += len;
    }
    0
}

/// check whether [`add`] is a valid SV39 VirtAddr or not.
fn is_valid_sv39_address(addr: usize) -> bool {
    // 提取第 38 位（从 0 开始计数）
    let bit38 = (addr >> 38) & 1;

    // 提取高位 [63:39]（共 25 位）
    let high_bits = addr >> 39;

    // 检查高位是否全为 bit38 的值
    // 如果 bit38 是 0，则高位应该全为 0
    // 如果 bit38 是 1，则高位应该全为 1
    if bit38 == 0 {
        high_bits == 0
    } else {
        // 0x1FFFFFF 是 25 位全为 1 的值
        high_bits == 0x1FFFFFF
    }
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // 检查地址是否符合 SV39 规范
    if !is_valid_sv39_address(start) {
        return -1; // 返回错误码
    }
    println!("[KERNEL] sys_map");
    new_memory(start, len, port)
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_munmap", current_task().unwrap().pid.0);
    // 检查地址是否符合 SV39 规范
    if !is_valid_sv39_address(start) {
        return -1; // 返回错误码
    }
    println!("[KERNEL] sys_munmap");
    free_memory(start, len)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);

    if let Some(elf_data) = get_app_data_by_name(path.as_str()) {
        let current_task = current_task().unwrap();
        // 创建新任务而不是替换当前任务
        if let Some(new_task) = current_task.spawn(elf_data) {
            let new_pid = new_task.pid.0;
            add_task(new_task);
            new_pid as isize // 返回新进程的PID
        } else {
            -1
        }
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    if prio >= 2 {
        let current_task = current_task().unwrap();
        current_task.set_priority(prio);
        prio
    } else {
        // prio不合法
        -1
    }
}
