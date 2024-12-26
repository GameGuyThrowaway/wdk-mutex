//! An idiomatic Rust mutex type for Windows kernel driver development. 
//! 
//! The crate will safely check IRQL before doing operations which would cause a STOP CODE of 
//! IRQL NOT LESS OR EQUAL (except for RAII dropping of scoped Mutex Guards).
//! In those cases, the API will return an error of the internal type `DriverMutexError`.
//! 
//! This crate is a work in progress to implement additional mutex types and functionality as required. Contributions, issues,
//! and discussions are welcome.
//! 
//! This crate is **NOT** affiliated with the WDK crates provided by Microsoft, but is designed to work with them for Windows Rust Kernel Driver
//! development.
//! 
//! # Additional features:
//! 
//! - `debug`: Enabling this feature will print debug messages to an attached debugger or kernel message viewer when an IRQL error occurs.
//! 
//! # Planned updates
//! 
//! ## Global interface:
//! 
//! A future addition is planned which will make the API more flexible for dynamically managing globally available 
//! mutexes to somewhat reduce the overhead required to use this crate.
//! 
//! ## Critical Sections:
//! 
//! An idiomatic implementation for entering and leaving a mutex critical section where no underlying T is protected.
//! 
//! ## FAST_MUTEX
//! 
//! An idiomatic implementation for FAST_MUTEX.
//! 
//! # Tests
//! 
//! Tests have been conducted on public modules.
//! 
//! No tests are included in the crate. Tests have been conducted on another project <https://github.com/0xflux/Sanctum>; but a new repo
//! will be created specifically for testing this crate which can be built as a driver.
//! 
//! <section class="warning">
//! As this crate integrates into the wdk ecosystem, Microsoft stipulate: This project is still in early stages of development and 
//! is not yet recommended for production use.
//! 
//! This is licenced with an MIT Licence, conditions can be found in LICENCE in the crate GitHub repository.
//! </section>

#![no_std]

pub mod kmutex;
// mod fast_mutex;

pub mod errors;