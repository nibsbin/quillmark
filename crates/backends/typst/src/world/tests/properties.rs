//! Font files over generated collection headers: whatever face count a header
//! declares, loading reads no more faces than the file holds offsets for.

use proptest::prelude::*;
use typst::foundations::Bytes;

use crate::world::faces;

proptest! {
    #[test]
    fn a_collection_yields_at_most_the_faces_its_length_holds(
        count in prop_oneof![Just(u32::MAX), any::<u32>()],
        body in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let mut data = b"ttcf".to_vec();
        data.extend(0x0001_0000u32.to_be_bytes());
        data.extend(count.to_be_bytes());
        data.extend(&body);
        prop_assert!(faces(Bytes::new(data)).count() <= body.len() / 4);
    }
}
