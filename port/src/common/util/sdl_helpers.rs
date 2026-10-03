//! Port of src/common/util/SdlHelpers.cpp
//!
//! The C++ `unique_ptr` deleters are not needed: the port owns SDL objects through rust-sdl2 types or frees raw pointers explicitly.
