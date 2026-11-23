// Attributes a PDF may keep in the parent `Pages` node instead of the page itself.

use lopdf::{Document, Object, ObjectId};

pub(super) const INHERITED: &[&[u8]] = &[b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// Walks the page tree up until the key is found.
pub(super) fn inherited_value(doc: &Document, start: ObjectId, key: &[u8]) -> Option<Object> {
    let mut current = Some(start);
    while let Some(id) = current {
        let object = doc.get_object(id).ok()?;
        let dict = object.as_dict().ok()?;
        if let Ok(value) = dict.get(key) {
            return Some(value.clone());
        }
        current = dict
            .get(b"Parent")
            .ok()
            .and_then(|object| object.as_reference().ok());
    }
    None
}
