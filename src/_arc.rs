//! Deprecated but leaving for reference as some of this logic may be useful in a
//! future planned addition.

use core::{ops::Deref, ptr::{drop_in_place, write}, sync::atomic::AtomicUsize};

use wdk::println;
use wdk_sys::{ntddk::{ExAllocatePool2, ExFreePool}, POOL_FLAG_NON_PAGED};

use crate::errors::DriverMutexError;

/// An atomically reference counted Arc in the non-paged pool
#[derive(Debug)]
pub struct ArcNP<T> {
    /// A pointer to the actual allocation, which holds the reference count and T
    ptr: *mut ArcInner<T>,
}

#[repr(C, align(8))]
#[derive(Debug)]
struct ArcInner<T> {
    /// The atomic reference count
    refcount: AtomicUsize,
    data: T,
}

impl<T> ArcNP<T> {
    /// Allocates a new atomically reference counted smart pointer in the 
    /// NonPagedPool with a given tag.
    pub fn new(data: T, tag: u32) -> Result<Self, DriverMutexError> {
        
        // 
        // Calculate the size required for the non-paged pool allocation and 
        // then allocate.
        //

        let inner_size = size_of::<ArcInner<T>>();

        let mem = unsafe {
            ExAllocatePool2(POOL_FLAG_NON_PAGED, inner_size as u64, tag)
        };

        if mem.is_null() {
            return Err(DriverMutexError::PagedPoolAllocFailed);
        }

        // Cast the memory allocation to our type
        let ptr = mem as *mut ArcInner<T>;

        //
        // write the ArcInner<T> into the newly allocated memory
        //

        // SAFETY: A null pointer check above ensures this operation is writing to properly 
        // initialised memory.
        unsafe {
            write(
                ptr, 
                ArcInner {
                    refcount: AtomicUsize::new(1),
                    data,
                }
            );
        }

        Ok(Self { ptr })
    }
}

impl<T> Deref for ArcNP<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: This is safe as the type keeps track of the validity of the reference
        // via 
        unsafe {
            &(*self.ptr).data
        }
    }
}

impl<T> Clone for ArcNP<T> {
    fn clone(&self) -> Self {

        // increment the reference count
        // SAFETY this operation is safe as Self's memory is tracked by the implementation of our smart pointer
        unsafe {
            let _ = &(*self.ptr).refcount.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        };

        // Return the underlying ArcNP
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for ArcNP<T> {
    fn drop(&mut self) {

        // dec the ref count
        let count_prior_to_dec = unsafe {
            &(*self.ptr).refcount
        }.fetch_sub(1, core::sync::atomic::Ordering::SeqCst);

        println!("[wdk-mutex] Dec val: {}...", count_prior_to_dec);

        // if the new count == 0, then we need to clean up the memory, else, there are still
        // valid references living.
        if count_prior_to_dec == 1 {
            // SAFETY: At this point we are operating on the final Arc lifetime, so the data is still
            // valid (as the count was 1, now 0 as it leaves its scope or is otherwise dropped).
            println!("[wdk-mutex] Dropping underlying memory...");
            unsafe {
                // drop the underlying data
                drop_in_place(&mut (*self.ptr).data);

                ExFreePool(self.ptr as *mut _);
            }
        }
    }
}