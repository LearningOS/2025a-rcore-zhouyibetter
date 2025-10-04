//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next, TASK_MANAGER},
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
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    // println!("DBG: us(hex)={:#x}, us(dec)={}", us, us);
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
        // println!("[DEBUG] ts.sec = {}, ts.usec = {}", (*ts).sec, (*ts).usec);
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");

    match trace_request {
        0 => unsafe {
            let p = id as *const u8;
            core::ptr::read_volatile(p) as isize
        },
        1 => unsafe {
            let p = id as *mut u8;
            core::ptr::write_volatile(p, data as u8);
            0
        },
        2 => TASK_MANAGER.count_syscall(id) as isize,
        _ => -1,
    }
}
