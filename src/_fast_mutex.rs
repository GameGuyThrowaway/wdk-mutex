//! This module is out of commission pending a future release, and is a work in progress. 
//! 
//! Public interfaces disabled during development so it cannot be used by somebody using this crate.
//! 
//! Use the `KMutex` module of this crate in the meantime.

#[cfg(not(test))]
extern crate wdk_panic;

use core::{cell::UnsafeCell, ops::{Deref, DerefMut}};
use wdk::println;
use wdk_sys::{ntddk::{ExAcquireFastMutex, ExReleaseFastMutex, KeGetCurrentIrql, KeInitializeEvent}, DISPATCH_LEVEL, FALSE, FAST_MUTEX, FM_LOCK_BIT, _EVENT_TYPE::SynchronizationEvent, APC_LEVEL};

use crate::errors::DriverMutexError;

#[allow(non_snake_case)]
unsafe fn ExInitializeFastMutex(kmutex: *mut FAST_MUTEX) {
    // check IRQL
    let irql = unsafe { KeGetCurrentIrql() };
    assert!(irql as u32 <= DISPATCH_LEVEL);

    core::ptr::write_volatile(&mut (*kmutex).Count, FM_LOCK_BIT as i32);

    (*kmutex).Owner = core::ptr::null_mut();
    (*kmutex).Contention = 0;
    KeInitializeEvent(&mut (*kmutex).Event, SynchronizationEvent, FALSE as _)
}

#[derive(Debug)]
struct FastMutex<T> {
    mutex: UnsafeCell<FAST_MUTEX>,
    inner: UnsafeCell<T>,
}

unsafe impl<T: Sync> Sync for FastMutex<T>{}

impl<T> FastMutex<T> {

    pub fn new(data: T) -> Result<Self, DriverMutexError> {
        if unsafe { KeGetCurrentIrql() } > DISPATCH_LEVEL as u8 {
            return Err(DriverMutexError::IrqlTooHigh)
        }

        let mut mutex = FAST_MUTEX::default();
        unsafe { ExInitializeFastMutex(&mut mutex) };
        let c = UnsafeCell::new(mutex);

        Ok(FastMutex {
            mutex: c,
            inner: UnsafeCell::new(data),
        })
    }


    pub fn lock(&self) -> Result<FastMutexGuard<'_, T>, DriverMutexError> {
        if unsafe { KeGetCurrentIrql() } > APC_LEVEL as u8 {
            return Err(DriverMutexError::IrqlTooHigh);
        }

        unsafe { ExAcquireFastMutex(self.mutex.get()) };
         
        Ok(FastMutexGuard {
            fast_mutex: self
        })
    }
}

#[derive(Debug)]
struct FastMutexGuard<'a, T> {
    fast_mutex: &'a FastMutex<T>,
}


impl<'a, T> Deref for FastMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.fast_mutex.inner.get() }
    }
}

impl<'a, T> DerefMut for FastMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.fast_mutex.inner.get() }
    }
}

impl<'a, T> Drop for FastMutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { ExReleaseFastMutex(self.fast_mutex.mutex.get()) }; 

    }
}