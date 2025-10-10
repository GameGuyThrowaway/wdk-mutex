//! An idiomatic Rust mutex type for Windows kernel driver development.
//!
//! The crate will safely check IRQL before doing operations which would cause a STOP CODE of
//! IRQL NOT LESS OR EQUAL (**except for RAII dropping of scoped Mutex Guards**).
//! In those cases, the API will return an error of the internal type `DriverMutexError`.
//!
//! This crate is a work in progress to implement additional mutex types and functionality as required. Contributions, issues,
//! and discussions are welcome.
//!
//! This crate is **not** affiliated with the WDK crates provided by Microsoft, but is designed to work with them for Windows Rust Kernel Driver
//! development.
//!
//! # Features:
//! - `driver-wdm`: The **default** features for this crate support WDM bindings.
//! - `driver-kmdf`: Compiles for the **Kernel Mode Driver Framework (KMDF)**.
//! - `driver-umdf`: Compiles for the **User Mode Driver Framework (UMDF)**.
//!
//! ## Enabling driver model features
//! Exactly **one** driver model feature must be active at compile time.
//! The crate defaults to **WDM**, but you can explicitly select another framework by changing the feature flags when you build or add the dependency.
//! 
//! ### Using `cargo add`
//! ```PowerShell
//! # Default (WDM)
//! cargo add wdk-mutex
//! 
//! # Kernel Mode Driver Framework (KMDF)
//! cargo add wdk-mutex --no-default-features --features driver-kmdf
//! 
//! # User Mode Driver Framework (UMDF)
//! cargo add wdk-mutex --no-default-features --features driver-umdf
//! ```
//! 
//! # Planned updates
//!
//! - **Critical Sections**: An idiomatic implementation for entering and leaving a mutex critical section where no underlying T is protected.
//!
//! # Tests
//!
//! Tests have been conducted on public modules.
//!
//! All tests are carried out at [wdk_mutex_tests](https://github.com/0xflux/wdk_mutex_tests),
//! a separate repository which is built as a driver to test all functionality of the `wdk-mutex` crate. If you wish to run the tests
//! yourself, or contribute, please check that repository.
//!
//! <section class="warning">
//! As this crate integrates into the wdk ecosystem, Microsoft stipulate: This project is still in early stages of development and
//! is not yet recommended for production use.
//!
//! This is licenced with an MIT Licence, conditions can be found in LICENCE in the crate GitHub repository.
//! </section>

#![no_std]

//
// Public modules
//
pub mod errors;
pub mod grt;
pub mod kmutex;
pub mod fast_mutex;

//
// Private modules
//
mod alloc;
