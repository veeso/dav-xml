# dav-xml

`WebDAV` ([RFC 4918](https://www.rfc-editor.org/rfc/rfc4918)) XML elements,
properties and (de)serialization for Rust.

## Usage

```rust,no_run
use dav_xml::FromXml;
use dav_xml::elements::{Multistatus, Response};

let xml = std::fs::read("multistatus.xml")?;
let multistatus = Multistatus::from_xml(xml)?;
for response in &multistatus.response {
    if let Response::Propstat { href, propstat, .. } = response {
        for propstat in propstat {
            if let Some(Some(Ok(size))) = propstat.prop.getcontentlength() {
                println!("{href} is {size} bytes", href = href, size = size.0);
            }
        }
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## RFC 4918 coverage

| Section | Items                              | Status |
| ------- | ---------------------------------- | ------ |
| 14      | All 30 XML elements                | Full   |
| 15      | All 10 live properties             | Full   |
| 16      | All 7 pre- and postcondition codes | Full   |

### Not covered

- RFC 3744
- RFC 4331
- RFC 3253
- `CalDAV`
- `CardDAV`

## Custom properties

Implement `Element`, `TryFrom<&Value>` and `Into<Value>` for custom `WebDAV`
properties:

```rust
use dav_xml::{Element, Error, Result, Value};

#[derive(Debug, PartialEq)]
struct QuotaUsedBytes(u64);

impl Element for QuotaUsedBytes {
    const NAMESPACE: &'static str = dav_xml::DAV_NAMESPACE;
    const PREFIX: &'static str = dav_xml::DAV_PREFIX;
    const LOCAL_NAME: &'static str = "quota-used-bytes";
}

impl TryFrom<&Value> for QuotaUsedBytes {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self> {
        value
            .as_str_of::<Self>()?
            .parse()
            .map(Self)
            .map_err(Error::invalid::<Self>)
    }
}

impl From<QuotaUsedBytes> for Value {
    fn from(value: QuotaUsedBytes) -> Self {
        value.0.to_string().into()
    }
}
```

## License

MIT OR Apache-2.0. The crate derives from
[`webdav-xml`](https://codeberg.org/d-k-bo/webdav-xml) by d-k-bo.
