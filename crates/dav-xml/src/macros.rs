// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Macros generating boilerplate for simple elements.

/// Define an empty `DAV:` element such as `<D:collection/>`.
macro_rules! unit_element {
    ($(#[$meta:meta])* $name:ident, $local_name:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct $name;

        impl $crate::Element for $name {
            const NAMESPACE: &'static str = $crate::DAV_NAMESPACE;
            const PREFIX: &'static str = $crate::DAV_PREFIX;
            const LOCAL_NAME: &'static str = $local_name;
        }

        impl TryFrom<&$crate::Value> for $name {
            type Error = $crate::Error;

            fn try_from(value: &$crate::Value) -> ::std::result::Result<Self, Self::Error> {
                match value {
                    $crate::Value::Empty => Ok(Self),
                    $crate::Value::Map(map) if map.is_empty() => Ok(Self),
                    _ => Err($crate::Error::InvalidValueType {
                        element: $local_name,
                        expected: "an empty element",
                    }),
                }
            }
        }

        impl From<$name> for $crate::Value {
            fn from(_: $name) -> $crate::Value {
                $crate::Value::Empty
            }
        }
    };
}
