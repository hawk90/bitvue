#![no_main]

use bitvue_protocol::{read_frame, write_frame, CancelParams, HelloParams, Request, Response};
use libfuzzer_sys::fuzz_target;

// Fuzz target for the sidecar wire protocol: the bytes `bitvue-desktop` writes to the sidecar's
// stdin. Every frame the reader accepts must survive a write/read round trip, and every payload
// must be refused or parsed without a panic.
fuzz_target!(|data: &[u8]| {
    let mut stream = data;
    while let Ok(Some((header, payload))) = read_frame(&mut stream) {
        let mut wire = Vec::new();
        write_frame(&mut wire, header.kind, header.correlation_id, &payload).unwrap();
        let (again, again_payload) = read_frame(&mut wire.as_slice()).unwrap().unwrap();
        assert_eq!(again.kind, header.kind);
        assert_eq!(again.correlation_id, header.correlation_id);
        assert_eq!(again_payload, payload);

        // What the sidecar does with a Control payload, and the other wire shapes.
        if let Ok(request) = serde_json::from_slice::<Request>(&payload) {
            let _ = serde_json::from_value::<CancelParams>(request.params.clone());
            let _ = serde_json::from_value::<HelloParams>(request.params);
        }
        let _ = serde_json::from_slice::<Response>(&payload);
    }
});
