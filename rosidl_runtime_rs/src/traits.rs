// Copyright 2020 DCS Corporation, All Rights Reserved.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// DISTRIBUTION A. Approved for public release; distribution unlimited.
// OPSEC #4584.
//
use std::borrow::Cow;
use std::fmt::Debug;

/// Internal trait that connects a particular `Sequence<T>` instance to generated C functions
/// that allocate and deallocate memory.
///
/// User code never needs to call these trait methods, much less implement this trait.
pub trait SequenceAlloc: Sized {
    /// Wraps the corresponding init function generated by `rosidl_generator_c`.
    fn sequence_init(seq: &mut crate::Sequence<Self>, size: usize) -> bool;
    /// Wraps the corresponding fini function generated by `rosidl_generator_c`.
    fn sequence_fini(seq: &mut crate::Sequence<Self>);
    /// Wraps the corresponding copy function generated by `rosidl_generator_c`.
    fn sequence_copy(in_seq: &crate::Sequence<Self>, out_seq: &mut crate::Sequence<Self>) -> bool;
}

/// Trait for RMW-native messages.
///
/// See the documentation for the [`Message`] trait, which is the trait that should generally be
/// used by user code.
///
/// User code never needs to call this trait's method, much less implement this trait.
pub trait RmwMessage: Clone + Debug + Default + Send + Sync + Message {
    /// A string representation of this message's type, e.g. "geometry_msgs/msg/Twist"
    const TYPE_NAME: &'static str;

    /// Get a pointer to the correct `rosidl_message_type_support_t` structure.
    fn get_type_support() -> usize;
}

/// Trait for types that can be used in a `rclrs::Subscription` and a `rclrs::Publisher`.
///
/// `rosidl_generator_rs` generates two types of messages that implement this trait:
/// - An "idiomatic" message type, in the `${package_name}::msg` module
/// - An "RMW-native" message type, in the `${package_name}::msg::rmw` module
///
/// # Idiomatic message type
/// The idiomatic message type aims to be familiar to Rust developers and ROS 2 developers coming
/// from `rclcpp`.
/// To this end, it translates the original ROS 2 message into a version that uses idiomatic Rust
/// structs: [`std::vec::Vec`] for sequences and [`std::string::String`] for strings. All other
/// fields are the same as in an RMW-native message.
///
/// This conversion incurs some overhead when reading and publishing messages.
///
/// It's possible to use the idiomatic type for a publisher and the RMW-native type for a
/// corresponding subscription, and vice versa.
///
/// # RMW-native message type
/// The RMW-native message type aims to achieve higher performance by avoiding the conversion
/// step to an idiomatic message.
///
/// It uses the following type mapping:
///
/// | Message field type | Rust type |
/// |------------|---------------|
/// | `string` | [`String`](crate::String) |
/// | `wstring` | [`WString`](crate::WString) |
/// | `string<=N`, for example `string<=10` | [`BoundedString`](crate::BoundedString) |
/// | `wstring<=N`, for example `wstring<=10` | [`BoundedWString`](crate::BoundedWString) |
/// | `T[]`, for example `int32[]` | [`Sequence`](crate::Sequence) |
/// | `T[<=N]`, for example `int32[<=32]` | [`BoundedSequence`](crate::BoundedSequence) |
/// | `T[N]`, for example `float32[8]` | standard Rust arrays |
/// | primitive type, for example `float64` | corresponding Rust primitive type |
///
/// <br/>
///
/// The linked Rust types provided by this package are equivalents of types defined in C that are
/// used by the RMW layer.
///
/// The API for these types, and the message as a whole, is still memory-safe and as convenient as
/// possible.
/// For instance, the [`Sequence`](crate::Sequence) struct that is used for sequences supports
/// iteration and all of the functionality of slices. However, it doesn't have an equivalent of
/// [`Vec::push()`], among others.
///
/// ## What does "RMW-native" mean in detail?
/// The message can be directly passed to and from the RMW layer because (1) its layout is
/// identical to the layout of the type generated by `rosidl_generator_c` and (2) the dynamic
/// memory inside the message is owned by the C allocator.
///
/// The above type mapping, together with a `#[repr(C)]` annotation on the message, guarantees
/// these two properties.
///
/// This means the user of a message does not need to care about memory ownership, because that is
/// managed by the relevant functions and trait impls.
///
/// ## I need even more detail, please
/// `rosidl_runtime_c` and the code generated by `rosidl_generator_c` manages
/// memory by means of four functions for each message: `init()`, `fini()`, `create()`, and
/// `destroy()`.
///
/// `init()` does the following:
/// - for a message, it calls `init()` on all its members that are of non-primitive type, and applies default values
/// - for a primitive sequence, it allocates the space requested
/// - for a string, it constructs a string containing a single null terminator byte
/// - for a non-primitive sequence, it zero-allocates the space requested and calls `init()` on all its elements
///
/// `fini()` does the following (which means after a call to `fini()`, everything inside the message has been deallocated):
/// - for a message, it calls `fini()` on all its members that are of non-primitive type
/// - for a primitive sequence, it deallocates
/// - for a string, it deallocates
/// - for a non-primitive sequence, it calls `fini()` on all its elements, and then deallocates
///
/// `create()` simply allocates space for the message itself, and calls `init()`.
///
/// `destroy()` simply deallocates the message itself, and calls `fini()`.
///
/// Memory ownership by C is achieved by calling `init()` when any string or sequence is created,
/// as well as in the `Default` impl for messages.
/// User code can still create messages explicitly, which will not call `init()`, but this is not a
///  problem, since nothing is allocated this way.
/// The `Drop` impl for any sequence or string will call `fini()`.

pub trait Message: Clone + Debug + Default + 'static + Send + Sync {
    /// The corresponding RMW-native message type.
    type RmwMsg: RmwMessage;

    /// Converts the idiomatic message into an RMW-native message.
    ///
    /// If the idiomatic message is owned, a slightly more efficient conversion is possible.
    /// This is why the function takes a `Cow`.
    ///
    /// If this function receives a borrowed message that is already RMW-native, it should
    /// directly return that borrowed message.
    /// This is why the return type is also `Cow`.
    fn into_rmw_message(msg_cow: Cow<'_, Self>) -> Cow<'_, Self::RmwMsg>;

    /// Converts the RMW-native message into an idiomatic message.
    fn from_rmw_message(msg: Self::RmwMsg) -> Self;
}

/// Trait for services.
///
/// User code never needs to call this trait's method, much less implement this trait.
pub trait Service: 'static {
    /// The request message associated with this service.
    type Request: Message;

    /// The response message associated with this service.
    type Response: Message;

    /// Get a pointer to the correct `rosidl_service_type_support_t` structure.
    fn get_type_support() -> usize;
}
