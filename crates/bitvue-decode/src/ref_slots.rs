//! Which frame type each of AV1's eight reference slots holds, so that a `show_existing_frame`
//! packet (which only names a slot) can still say what type the picture it shows has.
//!
//! The tracker is a pure function of the frame headers it is fed, in order; it knows nothing about
//! dav1d. Wherever it cannot be sure it answers `None` rather than guess: a slot it never saw
//! written, a header that did not parse (`forget`), a packet that shows no frame or several.

use crate::decoder::FrameType;

/// What the tracker needs from one frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeaderEvent {
    /// `show_existing_frame`: shows the frame held in `slot`.
    ShowExisting { slot: u8 },
    /// A coded frame. `refresh` is its `refresh_frame_flags` (bit `i` set: slot `i` takes this
    /// frame), `None` when the parser did not get it.
    Coded {
        frame_type: FrameType,
        shown: bool,
        refresh: Option<u8>,
    },
}

/// The frame type held by each reference slot; `None` where it is not known.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct RefSlotTypes {
    slots: [Option<FrameType>; 8],
}

impl RefSlotTypes {
    /// Drops everything known: after a reset, a flush, a failed send or an unreadable header the
    /// slots may no longer hold what was recorded.
    pub(crate) fn forget(&mut self) {
        self.slots = [None; 8];
    }

    /// Applies the frame headers of one packet in order and returns the type of the picture the
    /// packet shows, when it shows exactly one and its type is known.
    pub(crate) fn apply(&mut self, events: &[HeaderEvent]) -> Option<FrameType> {
        let mut shown = 0;
        let mut picture = None;
        for event in events {
            match *event {
                HeaderEvent::ShowExisting { slot } => {
                    let held = self.slots.get(slot as usize).copied().flatten();
                    shown += 1;
                    picture = held;
                    // Showing a key frame again refreshes every slot with it (spec 7.21).
                    if held == Some(FrameType::Key) {
                        self.slots = [Some(FrameType::Key); 8];
                    }
                }
                HeaderEvent::Coded {
                    frame_type,
                    shown: is_shown,
                    refresh,
                } => {
                    if is_shown {
                        shown += 1;
                        picture = Some(frame_type);
                    }
                    match refresh {
                        Some(bits) => {
                            for (i, slot) in self.slots.iter_mut().enumerate() {
                                if bits >> i & 1 == 1 {
                                    *slot = Some(frame_type);
                                }
                            }
                        }
                        None => self.forget(),
                    }
                }
            }
        }
        if shown == 1 {
            picture
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use FrameType::{Inter, Intra, Key};

    fn coded(frame_type: FrameType, shown: bool, refresh: u8) -> HeaderEvent {
        HeaderEvent::Coded {
            frame_type,
            shown,
            refresh: Some(refresh),
        }
    }
    fn existing(slot: u8) -> HeaderEvent {
        HeaderEvent::ShowExisting { slot }
    }

    #[test]
    fn a_coded_frame_that_is_shown_is_the_packets_picture() {
        let mut t = RefSlotTypes::default();
        assert_eq!(t.apply(&[coded(Key, true, 0xFF)]), Some(Key));
        assert_eq!(t.apply(&[coded(Inter, true, 0x01)]), Some(Inter));
    }

    #[test]
    fn hidden_frames_before_the_shown_one_do_not_count() {
        let mut t = RefSlotTypes::default();
        let events = [
            coded(Key, false, 0x01),
            coded(Inter, false, 0x02),
            coded(Intra, true, 0x04),
        ];
        assert_eq!(t.apply(&events), Some(Intra));
    }

    #[test]
    fn a_repeated_frame_has_the_type_of_the_slot_it_names() {
        let mut t = RefSlotTypes::default();
        t.apply(&[coded(Key, true, 0xFF)]);
        // a hidden inter frame goes to slot 3, a hidden intra-only one to slot 5
        assert_eq!(t.apply(&[coded(Inter, false, 1 << 3)]), None);
        assert_eq!(t.apply(&[coded(Intra, false, 1 << 5)]), None);
        assert_eq!(t.apply(&[existing(3)]), Some(Inter));
        assert_eq!(t.apply(&[existing(5)]), Some(Intra));
        assert_eq!(t.apply(&[existing(0)]), Some(Key));
    }

    #[test]
    fn a_frame_refreshing_several_slots_fills_all_of_them() {
        let mut t = RefSlotTypes::default();
        t.apply(&[coded(Key, true, 0xFF)]);
        t.apply(&[coded(Inter, true, 0b1010_0001)]);
        // each slot is asked of its own copy: showing a key frame again would refresh them all
        for (slot, want) in [
            (0, Inter),
            (1, Key),
            (2, Key),
            (5, Inter),
            (6, Key),
            (7, Inter),
        ] {
            assert_eq!(
                t.clone().apply(&[existing(slot)]),
                Some(want),
                "slot {slot}"
            );
        }
    }

    #[test]
    fn a_slot_that_was_never_written_is_unknown() {
        let mut t = RefSlotTypes::default();
        assert_eq!(t.apply(&[existing(2)]), None);
        t.apply(&[coded(Inter, false, 1 << 2)]);
        assert_eq!(t.apply(&[existing(2)]), Some(Inter));
        assert_eq!(t.apply(&[existing(3)]), None);
        // a slot number outside the eight is not a slot
        assert_eq!(t.apply(&[existing(9)]), None);
    }

    #[test]
    fn showing_a_key_frame_again_refreshes_every_slot_with_it() {
        let mut t = RefSlotTypes::default();
        t.apply(&[coded(Inter, false, 0xFF)]);
        t.apply(&[coded(Key, false, 1 << 4)]); // a hidden key frame in slot 4
        assert_eq!(t.apply(&[existing(0)]), Some(Inter));
        assert_eq!(t.apply(&[existing(4)]), Some(Key));
        // every slot now holds the key frame
        for slot in 0..8 {
            assert_eq!(t.apply(&[existing(slot)]), Some(Key), "slot {slot}");
        }
    }

    #[test]
    fn showing_an_inter_frame_again_changes_no_slot() {
        let mut t = RefSlotTypes::default();
        t.apply(&[coded(Key, true, 0xFF)]);
        t.apply(&[coded(Inter, false, 1 << 1)]);
        let before = t.clone();
        assert_eq!(t.apply(&[existing(1)]), Some(Inter));
        assert_eq!(t, before);
    }

    #[test]
    fn a_packet_that_shows_nothing_or_several_pictures_has_no_type() {
        let mut t = RefSlotTypes::default();
        t.apply(&[coded(Key, true, 0xFF)]);
        assert_eq!(t.apply(&[coded(Inter, false, 0x01)]), None);
        assert_eq!(t.apply(&[coded(Inter, true, 0x01), existing(0)]), None);
        assert_eq!(
            t.apply(&[coded(Inter, true, 0x01), coded(Inter, true, 0x02)]),
            None
        );
        assert_eq!(t.apply(&[]), None);
    }

    #[test]
    fn the_slots_still_follow_a_packet_whose_picture_type_is_open() {
        let mut t = RefSlotTypes::default();
        t.apply(&[coded(Key, true, 0xFF)]);
        // two shown frames: no picture type, but both writes happened
        t.apply(&[coded(Inter, true, 1 << 1), coded(Intra, true, 1 << 2)]);
        assert_eq!(t.apply(&[existing(1)]), Some(Inter));
        assert_eq!(t.apply(&[existing(2)]), Some(Intra));
    }

    #[test]
    fn forgetting_and_a_missing_refresh_field_drop_everything_known() {
        let mut t = RefSlotTypes::default();
        t.apply(&[coded(Key, true, 0xFF)]);
        t.forget();
        assert_eq!(t.apply(&[existing(0)]), None);

        t.apply(&[coded(Key, true, 0xFF)]);
        // a frame whose refresh_frame_flags the parser did not get: which slots it wrote is unknown
        let unreadable = HeaderEvent::Coded {
            frame_type: Inter,
            shown: true,
            refresh: None,
        };
        assert_eq!(t.apply(&[unreadable]), Some(Inter));
        assert_eq!(t.apply(&[existing(0)]), None);
    }

    #[test]
    fn a_new_key_frame_restores_what_was_forgotten() {
        let mut t = RefSlotTypes::default();
        t.forget();
        t.apply(&[coded(Key, true, 0xFF)]);
        assert_eq!(t.apply(&[existing(6)]), Some(Key));
    }
}
