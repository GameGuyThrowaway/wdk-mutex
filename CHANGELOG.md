# 0.0.5

Version 0.0.5 introduces the Global Reference Tracker, and the `Grt` module which allows you to easily create, track, and use Mutex's for a Windows Kernel Driver across all threads and 
callbacks. This improves developer ergonomics in creating, tracking, dropping etc Mutex's throughout your drivers codebase.

# 0.0.4

Introduction of two new functions which allow the caller to get owned copy of the protected data (`T`):

- to_owned()
- to_owned_box()

