//! Process management syscalls
use core::mem::size_of;

use crate::{
    mm::translated_byte_buffer,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, free_memory, new_memory,
        suspend_current_and_run_next, trace_read, trace_write, TASK_MANAGER,
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
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
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
pub fn is_valid_sv39_address(addr: usize) -> bool {
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

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    // 检查地址是否符合 SV39 规范
    if !is_valid_sv39_address(id) {
        return -1; // 返回错误码
    }

    println!("[DEBUG] trace -> id = {:#x}", id);
    match trace_request {
        0 => trace_read(id),
        1 => trace_write(id, data),
        2 => TASK_MANAGER.count_syscall(id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    // 检查地址是否符合 SV39 规范
    if !is_valid_sv39_address(start) {
        return -1; // 返回错误码
    }
    new_memory(start, len, port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    // 检查地址是否符合 SV39 规范
    if !is_valid_sv39_address(start) {
        return -1; // 返回错误码
    }
    free_memory(start, len)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
