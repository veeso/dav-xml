// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Declares an element's namespace and tag
pub trait Element {
    /// XML namespace of the element, e.g. `DAV:`
    const NAMESPACE: &'static str;
    /// The prefix used to abbreviate the namespace, e.g. `d`
    const PREFIX: &'static str;
    /// The local name of the element (the name inside the namespace), e.g.
    /// `multistatus`
    const LOCAL_NAME: &'static str;

    /// Validate that this element can be serialized.
    ///
    /// # Errors
    ///
    /// Returns an element-specific error when the value cannot be represented
    /// as valid XML.
    fn validate(&self) -> crate::Result<()> {
        Ok(())
    }
}

pub(crate) trait ElementExt: Element {
    fn element_name<S>() -> ElementName<S>
    where
        S: From<&'static str>,
    {
        ElementName {
            namespace: Some(Self::NAMESPACE.into()),
            prefix: Some(Self::PREFIX.into()),
            local_name: Self::LOCAL_NAME.into(),
        }
    }
}

impl<T: Element> ElementExt for T {}

/// A qualified XML element name.
#[derive(Clone, Debug)]
pub struct ElementName<S = &'static str> {
    /// The namespace URI, if the element is namespaced.
    pub namespace: Option<S>,
    /// The source document's prefix, when one was present.
    pub prefix: Option<S>,
    /// The local or qualified element name.
    pub local_name: S,
}

impl indexmap::Equivalent<ElementName<bytestring::ByteString>> for ElementName<&str> {
    fn equivalent(&self, key: &ElementName<bytestring::ByteString>) -> bool {
        self.namespace == key.namespace.as_deref() && self.local_name == &*key.local_name
    }
}

impl<S: PartialEq> PartialEq<ElementName<S>> for ElementName<S> {
    fn eq(&self, other: &Self) -> bool {
        self.namespace == other.namespace && self.local_name == other.local_name
    }
}

impl PartialEq<ElementName<&str>> for ElementName<bytestring::ByteString> {
    fn eq(&self, other: &ElementName<&str>) -> bool {
        self.namespace.as_deref() == other.namespace && &*self.local_name == other.local_name
    }
}

impl<S: PartialEq> Eq for ElementName<S> {}

impl<S: std::hash::Hash> std::hash::Hash for ElementName<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (&self.namespace, &self.local_name).hash(state);
    }
}
