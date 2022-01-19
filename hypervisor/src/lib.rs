#![no_std]
#![feature(let_else)]
#![feature(decl_macro)]
#![feature(const_deref)]
#![feature(const_mut_refs)]
#![feature(const_ptr_as_ref)]
#![feature(const_trait_impl)]
#![feature(new_uninit)]
#![feature(allocator_api)]
#![feature(box_syntax)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use] extern crate static_assertions;

pub mod debug;
pub mod hook;
pub mod svm;
pub mod utils;
