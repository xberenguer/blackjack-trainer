//! Optional tablemates/deeper-shoe controls; the zero-extra-seat table path
//! continues through the accepted table renderer unchanged.
use super::*;
use crate::{
    cards::Hand,
    render::{Align, TextStyle, text_width},
    text::Text,
    widgets::{GOLD, MUTED, WHITE, Widgets},
};
use core::fmt::Write;
impl App {
    fn new_face_hands(
        before: &[Hand; 4],
        before_count: usize,
        after: &[Hand; 4],
        after_count: usize,
        epoch_changed: bool,
    ) -> [bool; 4] {
        let mut new = [false; 4];
        for index in 0..after_count {
            new[index] = after[index].count() > 0
                && (epoch_changed
                    || after[index].positions().iter().any(|position| {
                        !before
                            .iter()
                            .take(before_count)
                            .any(|h| h.positions().contains(position))
                    }));
        }
        new
    }
    fn queue_face_hands(&mut self, seat: u8, hands: [Hand; 4], show: [bool; 4], previous: Phase) {
        self.other_reveals = [None; 4];
        self.other_reveal_len = 0;
        self.other_reveal_at = 0;
        for (index, hand) in hands.into_iter().enumerate() {
            if show[index] {
                self.other_reveals[self.other_reveal_len as usize] = Some(SeatReveal {
                    seat,
                    hand_index: index as u8,
                    hand,
                });
                self.other_reveal_len += 1;
            }
        }
        self.other_reveal_clock = self.now;
        self.other_reveal_previous = previous;
    }
    pub(super) fn queue_other_faces(
        &mut self,
        seat: u8,
        before: crate::seats::OtherSeat,
        after: crate::seats::OtherSeat,
        before_shoe: u32,
    ) {
        let show = Self::new_face_hands(
            &before.hands,
            before.hand_count,
            &after.hands,
            after.hand_count,
            before_shoe != self.game.shoe_number,
        );
        self.queue_face_hands(seat, after.hands, show, Phase::OtherPlayers);
    }
    pub(super) fn queue_hidden_player_faces(
        &mut self,
        before: [Hand; 4],
        before_count: usize,
        before_shoe: u32,
        previous: Phase,
    ) {
        let show = Self::new_face_hands(
            &before,
            before_count,
            &self.game.hands,
            self.game.hand_count,
            before_shoe != self.game.shoe_number,
        );
        let visible = match self.game.phase {
            Phase::OtherPlayers => None,
            Phase::Player => Some(self.game.active),
            Phase::Result => Some(0),
            _ => Some(self.result_hand.min(self.game.hand_count.saturating_sub(1))),
        };
        // Preserve ordinary table frames/timing. Queue only when the renderer
        // would otherwise hide at least one newly drawn physical card.
        if show
            .iter()
            .enumerate()
            .any(|(index, &new)| new && Some(index) != visible)
        {
            self.queue_face_hands(self.game.player_seat, self.game.hands, show, previous);
        }
    }

    pub(super) fn handle_seat_tap(&mut self, grey: bool) -> bool {
        if self.view == View::Profile && !grey {
            match self.selected {
                3 => {
                    self.set(View::ProfileDepth);
                    self.selected = [50, 60, 70, 75, 80, 85, 90]
                        .iter()
                        .position(|&n| n == self.pending_rules.penetration)
                        .unwrap_or(0);
                    return true;
                }
                4 => {
                    self.pending_extra_seats = self.game.extra_seats;
                    self.pending_player_seat = self.game.player_seat;
                    self.set(View::TableSeats);
                    return true;
                }
                5 => {
                    self.set(View::SeatCards);
                    return true;
                }
                6 => {
                    if self.game.configure(self.pending_rules) {
                        self.cut_index = self.game.bounded(17).unwrap() as u8;
                        self.aid_mask = 0;
                        self.observe();
                        self.open_table()
                    } else {
                        self.notice("Profile unavailable; finish the round", View::Profile)
                    }
                    return true;
                }
                _ => {}
            }
        }
        if self.view == View::Menu
            && !grey
            && self.selected == self.extras().1 + 8
            && self.game.extra_seats > 0
            && self.reveal.is_none()
        {
            self.set(View::SeatCards);
            return true;
        }
        if self.view == View::TableSeats {
            if grey {
                self.selected = (self.selected + 1) % 5
            } else {
                match self.selected {
                    0 => {
                        self.pending_extra_seats = (self.pending_extra_seats + 1) % 7;
                        self.pending_player_seat =
                            self.pending_player_seat.min(self.pending_extra_seats)
                    }
                    1 => {
                        self.pending_player_seat =
                            (self.pending_player_seat + 1) % (self.pending_extra_seats + 1)
                    }
                    2 => {
                        if self
                            .game
                            .configure_seats(self.pending_extra_seats, self.pending_player_seat)
                        {
                            self.cut_index = self.game.bounded(17).unwrap() as u8;
                            self.observe();
                            self.open_table()
                        } else {
                            self.notice("Seating changes need a finished round", View::TableSeats)
                        }
                    }
                    3 => self.set(View::SeatCards),
                    _ => self.set(View::Profile),
                }
            }
            return true;
        }
        if self.view == View::SeatCards {
            if grey {
                self.page = (self.page + 1) % seat_page_count(self).max(1)
            } else {
                self.set(View::Menu)
            }
            return true;
        }
        false
    }
}
fn seat_hand(a: &App, seat: usize, hand: usize) -> Option<&Hand> {
    if seat == a.game.player_seat as usize {
        if hand < a.game.hand_count {
            Some(&a.game.hands[hand])
        } else {
            None
        }
    } else {
        let other = if seat < a.game.player_seat as usize {
            seat
        } else {
            seat - 1
        };
        let s = a.game.others.get(other)?;
        if hand < s.hand_count {
            Some(&s.hands[hand])
        } else {
            None
        }
    }
}
fn seat_page_count(a: &App) -> usize {
    let mut pages = 0;
    for seat in 0..a.game.seat_count() {
        for hand in 0..4 {
            if let Some(h) = seat_hand(a, seat, hand) {
                pages += h.count().div_ceil(4).max(1)
            }
        }
    }
    pages
}
fn page_hand(a: &App, page: usize) -> Option<(usize, usize, Hand, usize)> {
    let mut n = 0;
    for seat in 0..a.game.seat_count() {
        for hand in 0..4 {
            if let Some(h) = seat_hand(a, seat, hand) {
                let pages = h.count().div_ceil(4).max(1);
                if page < n + pages {
                    return Some((seat, hand, *h, (page - n) * 4));
                }
                n += pages
            }
        }
    }
    None
}
fn label(w: &mut Widgets<'_>, y: i32, s: &str, size: u8, color: u16) {
    let mut size = size;
    while size > 8 && text_width(s, size, true) > 224 {
        size -= 1
    }
    w.label(
        (8, y),
        s,
        TextStyle {
            size,
            bold: true,
            color,
            align: Align::Left,
        },
    );
}
pub(crate) fn draw(a: &mut App) -> bool {
    if a.view == View::Table
        && a.other_reveal_len != 0
        && let Some(reveal) = a.other_reveals[a.other_reveal_at as usize]
    {
        let mut w = Widgets {
            frame: &mut a.frame,
            renderer: &mut a.renderer,
        };
        let mut text = Text::<64>::new();
        if reveal.seat == a.game.player_seat {
            let _ = write!(text, "YOUR HAND {}", reveal.hand_index + 1);
        } else {
            let _ = write!(text, "SEAT {} PLAYS", reveal.seat + 1);
        }
        w.title(text.as_str(), None, true);
        text.clear();
        let _ = write!(text, "Hand {} / exposed cards", reveal.hand_index + 1);
        label(&mut w, 23, text.as_str(), 10, WHITE);
        for (i, c) in reveal.hand.cards().iter().rev().take(4).rev().enumerate() {
            w.renderer
                .card(w.frame, (8 + i as i32 * 56, 36), Some(*c), false)
        }
        w.footer(if reveal.seat == a.game.player_seat {
            "YOUR DRAW / HOLD GREY: MENU"
        } else {
            "TABLEMATE CARD / HOLD GREY: MENU"
        });
        if let Some(mode) = a.toast {
            w.mode_footer(mode)
        }
        return true;
    }

    if a.view == View::Table
        && let Some(reveal) = a.reveal
        && reveal.progress >= 5
    {
        let index = (reveal.progress - 5) as usize;
        if index < a.game.extra_seats as usize {
            let seat = if index < a.game.player_seat as usize {
                index
            } else {
                index + 1
            };
            let hand = a.game.others[index].hands[0];
            let mut w = Widgets {
                frame: &mut a.frame,
                renderer: &mut a.renderer,
            };
            let mut text = Text::<64>::new();
            let _ = write!(text, "SEAT {} / HAND 1", seat + 1);
            w.title(text.as_str(), None, true);
            text.clear();
            let _ = write!(
                text,
                "Initial cards / other seat {} of {}",
                index + 1,
                a.game.extra_seats
            );
            label(&mut w, 23, text.as_str(), 10, WHITE);
            for (i, c) in hand.cards().iter().take(2).enumerate() {
                w.renderer
                    .card(w.frame, (8 + i as i32 * 56, 36), Some(*c), false)
            }
            w.footer("INITIAL DEAL / WATCH ALL SEATS");
            if let Some(mode) = a.toast {
                w.mode_footer(mode)
            }
            return true;
        }
    }

    if a.view == View::ShoeInfo {
        let inventory = a.game.inventory();
        let mut w = Widgets {
            frame: &mut a.frame,
            renderer: &mut a.renderer,
        };
        let mut b = Text::<128>::new();
        w.title(
            "SHOE INFORMATION",
            Some("Public card quantities / no hidden faces"),
            false,
        );
        let _ = write!(
            b,
            "Shoe {} / {} decks / marker {}%",
            a.game.shoe_number, a.game.rules.decks, a.game.rules.penetration
        );
        label(&mut w, 33, b.as_str(), 12, WHITE);
        b.clear();
        let _ = write!(
            b,
            "Undealt {} / held {} / discard {}",
            inventory.unseen, inventory.held, inventory.discarded
        );
        label(&mut w, 57, b.as_str(), 11, WHITE);
        b.clear();
        let _ = write!(
            b,
            "Exceptional discard refills {}",
            a.game.emergency_shuffles
        );
        label(&mut w, 81, b.as_str(), 11, MUTED);
        label(
            &mut w,
            105,
            if a.game.needs_shuffle() {
                "Full shuffle before next round"
            } else {
                "Finish ordinary rounds before shuffle"
            },
            10,
            GOLD,
        );
        w.footer("BLUE: MENU");
        return true;
    }
    if a.view == View::Rules && a.page >= 5 {
        let mut w = Widgets {
            frame: &mut a.frame,
            renderer: &mut a.renderer,
        };
        w.title(
            if a.page == 5 {
                "OPTIONAL TABLEMATES"
            } else {
                "RARE SHOE EXHAUSTION"
            },
            None,
            false,
        );
        let lines = if a.page == 5 {
            [
                "Up to six other virtual seats.",
                "Basic strategy; flat minimum bets.",
                "Deal clockwise from dealer-left.",
                "Choose your place in the seating.",
                "Review every exposed table hand.",
            ]
        } else {
            [
                "If cards run out during a round:",
                "Shuffle only the discard rack.",
                "Held cards are always excluded.",
                "Burn one unseen; reset count epoch.",
                "Finish; full shuffle before next hand.",
            ]
        };
        for (i, line) in lines.iter().enumerate() {
            label(&mut w, 30 + i as i32 * 17, line, 11, WHITE)
        }
        w.footer("GREY: PAGE   BLUE: MENU");
        return true;
    }
    if !(matches!(a.view, View::TableSeats | View::SeatCards)
        || (a.view == View::Table && a.game.phase == Phase::OtherPlayers && a.reveal.is_none()))
    {
        return false;
    }
    let page = page_hand(a, a.page);
    let count = seat_page_count(a);
    let running = a.tracker.running_count();
    let tc = a.tracker.true_count(a.count_context().undealt);
    let counter = a.active_count_display();
    if a.view == View::Table && counter != 0 {
        a.mark_aid(if counter == 2 { 7 } else { 1 })
    }
    let mut w = Widgets {
        frame: &mut a.frame,
        renderer: &mut a.renderer,
    };
    let mut b = Text::<128>::new();
    match a.view {
        View::TableSeats => {
            w.title(
                "TABLE SEATING",
                Some("Optional basic-strategy virtual tablemates"),
                false,
            );
            let mut extra = Text::<48>::new();
            let _ = write!(extra, "Other seats: {}", a.pending_extra_seats);
            let mut place = Text::<48>::new();
            let _ = write!(
                place,
                "Your place: {} / {}",
                a.pending_player_seat + 1,
                a.pending_extra_seats + 1
            );
            w.list(
                &[
                    extra.as_str(),
                    place.as_str(),
                    "Apply seating / fresh shoe",
                    "Review table cards",
                    "Return to profile",
                ],
                a.selected,
                None,
            )
        }
        View::SeatCards => {
            w.title("VISIBLE TABLE CARDS", None, false);
            if let Some((seat, hand, h, start)) = page {
                let _ = write!(
                    b,
                    "Seat {}{} / hand {} / {}/{}",
                    seat + 1,
                    if seat == a.game.player_seat as usize {
                        " (YOU)"
                    } else {
                        ""
                    },
                    hand + 1,
                    a.page + 1,
                    count
                );
                label(&mut w, 23, b.as_str(), 10, GOLD);
                for (i, c) in h.cards().iter().skip(start).take(4).enumerate() {
                    w.renderer
                        .card(w.frame, (8 + i as i32 * 56, 36), Some(*c), false)
                }
            } else {
                label(&mut w, 51, "No table cards dealt yet", 15, WHITE)
            }
            w.footer("GREY: CARDS   BLUE: MENU")
        }
        View::Table => {
            let seat = a.game.active_seat as usize;
            let other = if seat < a.game.player_seat as usize {
                seat
            } else {
                seat.saturating_sub(1)
            };
            let other_seat = a.game.others[other];
            let hand = other_seat
                .active
                .min(other_seat.hand_count.saturating_sub(1));
            let h = other_seat.hands[hand];
            let _ = write!(b, "SEAT {} PLAYS", seat + 1);
            w.title(b.as_str(), None, true);
            b.clear();
            let _ = write!(b, "Hand {}", hand + 1);
            if counter != 0 {
                let _ = write!(b, " / RC {:+}", running);
                if counter == 2
                    && let Some(tc) = tc
                {
                    let _ = write!(b, " TC {:+.1}", tc);
                }
            }
            label(&mut w, 23, b.as_str(), 10, WHITE);
            for (i, c) in h.cards().iter().rev().take(4).rev().enumerate() {
                w.renderer
                    .card(w.frame, (8 + i as i32 * 56, 36), Some(*c), false)
            }
            w.footer("TABLEMATE TURN / HOLD GREY: MENU")
        }
        _ => {}
    }
    if let Some(mode) = a.toast {
        w.mode_footer(mode)
    }
    true
}

#[cfg(test)]
mod physical_face_tests {
    use super::*;
    #[test]
    fn splitting_and_index_shifts_do_not_reveal_known_cards_as_new() {
        let mut before = [Hand::EMPTY; 4];
        before[0].add(crate::cards::Card::from_id(0).unwrap(), 10);
        before[0].add(crate::cards::Card::from_id(13).unwrap(), 11);
        before[1].add(crate::cards::Card::from_id(2).unwrap(), 12);
        before[1].add(crate::cards::Card::from_id(4).unwrap(), 13);
        let mut after = [Hand::EMPTY; 4];
        after[0].add(before[0].cards()[0], 10);
        after[0].add(crate::cards::Card::from_id(0).unwrap(), 14);
        after[1].add(before[0].cards()[1], 11);
        after[2] = before[1];
        assert_eq!(
            App::new_face_hands(&before, 2, &after, 3, false),
            [true, false, false, false]
        );
    }
}
