//! Explicit owning-App storage schema. Regenerate only after reviewing every new field.
use super::*;
use crate::resume_private::{SnapshotCopy, Wire};
impl App {
    pub(crate) fn resume_write(
        &self,
        w: &mut crate::resume_private::Writer,
    ) -> crate::resume_private::Result<()> {
        let Self {
            session_analysis,
            session_metrics,
            analysis_last_round,
            analysis_base_wager,
            analysis_valid,
            deviation_practice,
            drill_target,
            drill_mix,
            drill_profile,
            drill_generator,
            drill_token,
            drill_choice,
            wager_practice,
            pending_extra_seats,
            pending_player_seat,
            wager_enabled,
            wager_policy,
            other_reveals,
            other_reveal_len,
            other_reveal_at,
            other_reveal_clock,
            other_reveal_previous,
            observed_emergency_shuffles,
            wager_token,
            wager_checkpoint,
            wager_evidence,
            wager_skip_once,
            wager_amount,
            game,
            frame: _,
            training,
            count_practice,
            round_display: _,
            background: _,
            card_back: _,
            brightness,
            flipped,
            sound_enabled,
            audio_volume: _,
            audio_mask: _,
            fast_mode: _,
            resume_enabled,
            checkpoint_revision,
            dirty: _,
            policy,
            count_display,
            guide_count_display,
            view,
            selected,
            tracker,
            renderer: _,
            controls: _,
            sounds: _,
            initial_seed,
            load_allowed: _,
            now,
            clock,
            cut_index,
            result_hand,
            return_view,
            notice_return,
            notice,
            pending_rules,
            minimum_edit,
            amount,
            page,
            card_group,
            card_page,
            pending,
            hint,
            live_ev,
            round_id,
            last_review,
            test_round,
            review_due,
            decision_counter,
            fingerprint,
            current_token,
            assisted_token,
            reveal,
            reveal_clock,
            toast,
            toast_clock,
            session,
            count_token,
            count_kind,
            count_answer,
            count_assisted,
            aid_key,
            aid_mask,
        } = self;
        session_analysis.put(w)?;
        session_metrics.put(w)?;
        analysis_last_round.put(w)?;
        analysis_base_wager.put(w)?;
        analysis_valid.put(w)?;
        deviation_practice.put(w)?;
        drill_target.put(w)?;
        drill_mix.put(w)?;
        drill_profile.put(w)?;
        drill_generator.put(w)?;
        drill_token.put(w)?;
        drill_choice.put(w)?;
        wager_practice.put(w)?;
        pending_extra_seats.put(w)?;
        pending_player_seat.put(w)?;
        wager_enabled.put(w)?;
        wager_policy.put(w)?;
        other_reveals.put(w)?;
        other_reveal_len.put(w)?;
        other_reveal_at.put(w)?;
        other_reveal_clock.put(w)?;
        other_reveal_previous.put(w)?;
        observed_emergency_shuffles.put(w)?;
        wager_token.put(w)?;
        wager_checkpoint.put(w)?;
        wager_evidence.put(w)?;
        wager_skip_once.put(w)?;
        wager_amount.put(w)?;
        game.put(w)?;
        training.put(w)?;
        count_practice.put(w)?;
        brightness.put(w)?;
        flipped.put(w)?;
        sound_enabled.put(w)?;
        resume_enabled.put(w)?;
        checkpoint_revision.put(w)?;
        policy.put(w)?;
        count_display.put(w)?;
        guide_count_display.put(w)?;
        view.put(w)?;
        selected.put(w)?;
        tracker.put(w)?;
        initial_seed.put(w)?;
        now.put(w)?;
        clock.put(w)?;
        cut_index.put(w)?;
        result_hand.put(w)?;
        return_view.put(w)?;
        notice_return.put(w)?;
        notice.put(w)?;
        pending_rules.put(w)?;
        minimum_edit.put(w)?;
        amount.put(w)?;
        page.put(w)?;
        card_group.put(w)?;
        card_page.put(w)?;
        pending.put(w)?;
        hint.put(w)?;
        live_ev.put(w)?;
        round_id.put(w)?;
        last_review.put(w)?;
        test_round.put(w)?;
        review_due.put(w)?;
        decision_counter.put(w)?;
        fingerprint.put(w)?;
        current_token.put(w)?;
        assisted_token.put(w)?;
        reveal.put(w)?;
        reveal_clock.put(w)?;
        toast.put(w)?;
        toast_clock.put(w)?;
        session.put(w)?;
        count_token.put(w)?;
        count_kind.put(w)?;
        count_answer.put(w)?;
        count_assisted.put(w)?;
        aid_key.put(w)?;
        aid_mask.put(w)?;
        Ok(())
    }
    pub(crate) fn resume_read(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            session_analysis,
            session_metrics,
            analysis_last_round,
            analysis_base_wager,
            analysis_valid,
            deviation_practice,
            drill_target,
            drill_mix,
            drill_profile,
            drill_generator,
            drill_token,
            drill_choice,
            wager_practice,
            pending_extra_seats,
            pending_player_seat,
            wager_enabled,
            wager_policy,
            other_reveals,
            other_reveal_len,
            other_reveal_at,
            other_reveal_clock,
            other_reveal_previous,
            observed_emergency_shuffles,
            wager_token,
            wager_checkpoint,
            wager_evidence,
            wager_skip_once,
            wager_amount,
            game,
            frame: _,
            training,
            count_practice,
            round_display: _,
            background: _,
            card_back: _,
            brightness,
            flipped,
            sound_enabled,
            audio_volume: _,
            audio_mask: _,
            fast_mode: _,
            resume_enabled,
            checkpoint_revision,
            dirty: _,
            policy,
            count_display,
            guide_count_display,
            view,
            selected,
            tracker,
            renderer: _,
            controls: _,
            sounds: _,
            initial_seed,
            load_allowed: _,
            now,
            clock,
            cut_index,
            result_hand,
            return_view,
            notice_return,
            notice,
            pending_rules,
            minimum_edit,
            amount,
            page,
            card_group,
            card_page,
            pending,
            hint,
            live_ev,
            round_id,
            last_review,
            test_round,
            review_due,
            decision_counter,
            fingerprint,
            current_token,
            assisted_token,
            reveal,
            reveal_clock,
            toast,
            toast_clock,
            session,
            count_token,
            count_kind,
            count_answer,
            count_assisted,
            aid_key,
            aid_mask,
        } = self;
        session_analysis.read_into(r)?;
        session_metrics.read_into(r)?;
        analysis_last_round.read_into(r)?;
        analysis_base_wager.read_into(r)?;
        analysis_valid.read_into(r)?;
        deviation_practice.read_into(r)?;
        drill_target.read_into(r)?;
        drill_mix.read_into(r)?;
        drill_profile.read_into(r)?;
        drill_generator.read_into(r)?;
        drill_token.read_into(r)?;
        drill_choice.read_into(r)?;
        wager_practice.read_into(r)?;
        pending_extra_seats.read_into(r)?;
        pending_player_seat.read_into(r)?;
        wager_enabled.read_into(r)?;
        wager_policy.read_into(r)?;
        other_reveals.read_into(r)?;
        other_reveal_len.read_into(r)?;
        other_reveal_at.read_into(r)?;
        other_reveal_clock.read_into(r)?;
        other_reveal_previous.read_into(r)?;
        observed_emergency_shuffles.read_into(r)?;
        wager_token.read_into(r)?;
        wager_checkpoint.read_into(r)?;
        wager_evidence.read_into(r)?;
        wager_skip_once.read_into(r)?;
        wager_amount.read_into(r)?;
        game.read_into(r)?;
        training.read_into(r)?;
        count_practice.read_into(r)?;
        brightness.read_into(r)?;
        flipped.read_into(r)?;
        sound_enabled.read_into(r)?;
        resume_enabled.read_into(r)?;
        checkpoint_revision.read_into(r)?;
        policy.read_into(r)?;
        count_display.read_into(r)?;
        guide_count_display.read_into(r)?;
        view.read_into(r)?;
        selected.read_into(r)?;
        tracker.read_into(r)?;
        initial_seed.read_into(r)?;
        now.read_into(r)?;
        clock.read_into(r)?;
        cut_index.read_into(r)?;
        result_hand.read_into(r)?;
        return_view.read_into(r)?;
        notice_return.read_into(r)?;
        notice.read_into(r)?;
        pending_rules.read_into(r)?;
        minimum_edit.read_into(r)?;
        amount.read_into(r)?;
        page.read_into(r)?;
        card_group.read_into(r)?;
        card_page.read_into(r)?;
        pending.read_into(r)?;
        hint.read_into(r)?;
        live_ev.read_into(r)?;
        round_id.read_into(r)?;
        last_review.read_into(r)?;
        test_round.read_into(r)?;
        review_due.read_into(r)?;
        decision_counter.read_into(r)?;
        fingerprint.read_into(r)?;
        current_token.read_into(r)?;
        assisted_token.read_into(r)?;
        reveal.read_into(r)?;
        reveal_clock.read_into(r)?;
        toast.read_into(r)?;
        toast_clock.read_into(r)?;
        session.read_into(r)?;
        count_token.read_into(r)?;
        count_kind.read_into(r)?;
        count_answer.read_into(r)?;
        count_assisted.read_into(r)?;
        aid_key.read_into(r)?;
        aid_mask.read_into(r)?;
        Ok(())
    }
    /// Copy a rendered checkpoint into separately owned frozen storage without a whole-App temporary.
    /// Frame/renderer/control/audio resources are intentionally not copied. Core owns no heap/resources.
    pub fn capture_clone_into(&self, out: &mut Self) -> crate::resume_private::Result<()> {
        if self.dirty || self.game.shoe_number == 0 || core::mem::needs_drop::<Self>() {
            return Err(crate::resume_private::Error::Inconsistent);
        }
        let Self {
            session_analysis,
            session_metrics,
            analysis_last_round,
            analysis_base_wager,
            analysis_valid,
            deviation_practice,
            drill_target,
            drill_mix,
            drill_profile,
            drill_generator,
            drill_token,
            drill_choice,
            wager_practice,
            pending_extra_seats,
            pending_player_seat,
            wager_enabled,
            wager_policy,
            other_reveals,
            other_reveal_len,
            other_reveal_at,
            other_reveal_clock,
            other_reveal_previous,
            observed_emergency_shuffles,
            wager_token,
            wager_checkpoint,
            wager_evidence,
            wager_skip_once,
            wager_amount,
            game,
            frame: _,
            training,
            count_practice,
            round_display,
            background,
            card_back,
            brightness,
            flipped,
            sound_enabled,
            audio_volume,
            audio_mask,
            fast_mode,
            resume_enabled,
            checkpoint_revision,
            dirty: _,
            policy,
            count_display,
            guide_count_display,
            view,
            selected,
            tracker,
            renderer: _,
            controls: _,
            sounds: _,
            initial_seed,
            load_allowed: _,
            now,
            clock,
            cut_index,
            result_hand,
            return_view,
            notice_return,
            notice,
            pending_rules,
            minimum_edit,
            amount,
            page,
            card_group,
            card_page,
            pending,
            hint,
            live_ev,
            round_id,
            last_review,
            test_round,
            review_due,
            decision_counter,
            fingerprint,
            current_token,
            assisted_token,
            reveal,
            reveal_clock,
            toast,
            toast_clock,
            session,
            count_token,
            count_kind,
            count_answer,
            count_assisted,
            aid_key,
            aid_mask,
        } = self;
        session_analysis.snapshot_copy_to(&mut out.session_analysis);
        session_metrics.snapshot_copy_to(&mut out.session_metrics);
        analysis_last_round.snapshot_copy_to(&mut out.analysis_last_round);
        analysis_base_wager.snapshot_copy_to(&mut out.analysis_base_wager);
        analysis_valid.snapshot_copy_to(&mut out.analysis_valid);
        deviation_practice.snapshot_copy_to(&mut out.deviation_practice);
        drill_target.snapshot_copy_to(&mut out.drill_target);
        drill_mix.snapshot_copy_to(&mut out.drill_mix);
        drill_profile.snapshot_copy_to(&mut out.drill_profile);
        drill_generator.snapshot_copy_to(&mut out.drill_generator);
        drill_token.snapshot_copy_to(&mut out.drill_token);
        drill_choice.snapshot_copy_to(&mut out.drill_choice);
        wager_practice.snapshot_copy_to(&mut out.wager_practice);
        pending_extra_seats.snapshot_copy_to(&mut out.pending_extra_seats);
        pending_player_seat.snapshot_copy_to(&mut out.pending_player_seat);
        wager_enabled.snapshot_copy_to(&mut out.wager_enabled);
        wager_policy.snapshot_copy_to(&mut out.wager_policy);
        other_reveals.snapshot_copy_to(&mut out.other_reveals);
        other_reveal_len.snapshot_copy_to(&mut out.other_reveal_len);
        other_reveal_at.snapshot_copy_to(&mut out.other_reveal_at);
        other_reveal_clock.snapshot_copy_to(&mut out.other_reveal_clock);
        other_reveal_previous.snapshot_copy_to(&mut out.other_reveal_previous);
        observed_emergency_shuffles.snapshot_copy_to(&mut out.observed_emergency_shuffles);
        wager_token.snapshot_copy_to(&mut out.wager_token);
        wager_checkpoint.snapshot_copy_to(&mut out.wager_checkpoint);
        wager_evidence.snapshot_copy_to(&mut out.wager_evidence);
        wager_skip_once.snapshot_copy_to(&mut out.wager_skip_once);
        wager_amount.snapshot_copy_to(&mut out.wager_amount);
        game.snapshot_copy_to(&mut out.game);
        training.snapshot_copy_to(&mut out.training);
        count_practice.snapshot_copy_to(&mut out.count_practice);
        round_display.snapshot_copy_to(&mut out.round_display);
        background.snapshot_copy_to(&mut out.background);
        card_back.snapshot_copy_to(&mut out.card_back);
        brightness.snapshot_copy_to(&mut out.brightness);
        flipped.snapshot_copy_to(&mut out.flipped);
        sound_enabled.snapshot_copy_to(&mut out.sound_enabled);
        audio_volume.snapshot_copy_to(&mut out.audio_volume);
        audio_mask.snapshot_copy_to(&mut out.audio_mask);
        fast_mode.snapshot_copy_to(&mut out.fast_mode);
        resume_enabled.snapshot_copy_to(&mut out.resume_enabled);
        checkpoint_revision.snapshot_copy_to(&mut out.checkpoint_revision);
        policy.snapshot_copy_to(&mut out.policy);
        count_display.snapshot_copy_to(&mut out.count_display);
        guide_count_display.snapshot_copy_to(&mut out.guide_count_display);
        view.snapshot_copy_to(&mut out.view);
        selected.snapshot_copy_to(&mut out.selected);
        tracker.snapshot_copy_to(&mut out.tracker);
        initial_seed.snapshot_copy_to(&mut out.initial_seed);
        now.snapshot_copy_to(&mut out.now);
        clock.snapshot_copy_to(&mut out.clock);
        cut_index.snapshot_copy_to(&mut out.cut_index);
        result_hand.snapshot_copy_to(&mut out.result_hand);
        return_view.snapshot_copy_to(&mut out.return_view);
        notice_return.snapshot_copy_to(&mut out.notice_return);
        notice.snapshot_copy_to(&mut out.notice);
        pending_rules.snapshot_copy_to(&mut out.pending_rules);
        minimum_edit.snapshot_copy_to(&mut out.minimum_edit);
        amount.snapshot_copy_to(&mut out.amount);
        page.snapshot_copy_to(&mut out.page);
        card_group.snapshot_copy_to(&mut out.card_group);
        card_page.snapshot_copy_to(&mut out.card_page);
        pending.snapshot_copy_to(&mut out.pending);
        hint.snapshot_copy_to(&mut out.hint);
        live_ev.snapshot_copy_to(&mut out.live_ev);
        round_id.snapshot_copy_to(&mut out.round_id);
        last_review.snapshot_copy_to(&mut out.last_review);
        test_round.snapshot_copy_to(&mut out.test_round);
        review_due.snapshot_copy_to(&mut out.review_due);
        decision_counter.snapshot_copy_to(&mut out.decision_counter);
        fingerprint.snapshot_copy_to(&mut out.fingerprint);
        current_token.snapshot_copy_to(&mut out.current_token);
        assisted_token.snapshot_copy_to(&mut out.assisted_token);
        reveal.snapshot_copy_to(&mut out.reveal);
        reveal_clock.snapshot_copy_to(&mut out.reveal_clock);
        toast.snapshot_copy_to(&mut out.toast);
        toast_clock.snapshot_copy_to(&mut out.toast_clock);
        session.snapshot_copy_to(&mut out.session);
        count_token.snapshot_copy_to(&mut out.count_token);
        count_kind.snapshot_copy_to(&mut out.count_kind);
        count_answer.snapshot_copy_to(&mut out.count_answer);
        count_assisted.snapshot_copy_to(&mut out.count_assisted);
        aid_key.snapshot_copy_to(&mut out.aid_key);
        aid_mask.snapshot_copy_to(&mut out.aid_mask);
        out.controls = Controls::EMPTY;
        out.sounds = SoundQueue::EMPTY;
        out.dirty = false;
        out.load_allowed = false;
        Ok(())
    }
}
impl Wire for View {
    fn initial() -> Self {
        Self::AnalysisMenu
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::AnalysisMenu),
            1 => Ok(Self::AnalysisSummary),
            2 => Ok(Self::AnalysisGraph),
            3 => Ok(Self::AnalysisRounds),
            4 => Ok(Self::AnalysisAccuracy),
            5 => Ok(Self::AnalysisCosts),
            6 => Ok(Self::AnalysisDecisions),
            7 => Ok(Self::AnalysisAssumptions),
            8 => Ok(Self::DeviationPractice),
            9 => Ok(Self::DeviationTarget),
            10 => Ok(Self::DeviationQuestion),
            11 => Ok(Self::DeviationContext),
            12 => Ok(Self::DeviationReference),
            13 => Ok(Self::DeviationResult),
            14 => Ok(Self::DeviationStats),
            15 => Ok(Self::DeviationHistory),
            16 => Ok(Self::DeviationPolicy),
            17 => Ok(Self::TableSeats),
            18 => Ok(Self::SeatCards),
            19 => Ok(Self::WagerPractice),
            20 => Ok(Self::WagerAnswer),
            21 => Ok(Self::WagerHelp),
            22 => Ok(Self::WagerResult),
            23 => Ok(Self::WagerStats),
            24 => Ok(Self::WagerHistory),
            25 => Ok(Self::WagerPolicy),
            26 => Ok(Self::Cut),
            27 => Ok(Self::Burn),
            28 => Ok(Self::Table),
            29 => Ok(Self::Menu),
            30 => Ok(Self::Wager),
            31 => Ok(Self::Amount),
            32 => Ok(Self::Training),
            33 => Ok(Self::TrainingStats),
            34 => Ok(Self::TrainingReview),
            35 => Ok(Self::RoundReview),
            36 => Ok(Self::Hint),
            37 => Ok(Self::CoachCheck),
            38 => Ok(Self::ActionEv),
            39 => Ok(Self::EvReview),
            40 => Ok(Self::CountPractice),
            41 => Ok(Self::CountAnswer),
            42 => Ok(Self::CountResult),
            43 => Ok(Self::CountStats),
            44 => Ok(Self::CountHistory),
            45 => Ok(Self::CountInfo),
            46 => Ok(Self::Settings),
            47 => Ok(Self::Profile),
            48 => Ok(Self::ProfilePreset),
            49 => Ok(Self::ProfileAmount),
            50 => Ok(Self::ProfileDepth),
            51 => Ok(Self::ShoeInfo),
            52 => Ok(Self::Rules),
            53 => Ok(Self::Session),
            54 => Ok(Self::Bankroll),
            55 => Ok(Self::Cards),
            56 => Ok(Self::Controls),
            57 => Ok(Self::Leave),
            58 => Ok(Self::Summary),
            59 => Ok(Self::Refill),
            60 => Ok(Self::Notice),
            61 => Ok(Self::Sound),
            62 => Ok(Self::AudioVolume),
            63 => Ok(Self::Display),
            64 => Ok(Self::Brightness),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}
impl Wire for SeatReveal {
    fn initial() -> Self {
        Self {
            seat: 0,
            hand_index: 0,
            hand: crate::cards::Hand::EMPTY,
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        self.seat.put(w)?;
        self.hand_index.put(w)?;
        self.hand.put(w)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            seat: Wire::get(r)?,
            hand_index: Wire::get(r)?,
            hand: Wire::get(r)?,
        })
    }
}
impl App {
    pub(crate) fn resume_now(&self) -> u32 {
        self.now
    }
    pub fn resume_session_id(&self) -> u64 {
        self.session
    }
    pub(crate) fn resume_valid(&self) -> bool {
        if !self.session_analysis.resume_valid()
            || !self.session_metrics.resume_valid()
            || !self.game.resume_valid()
            || !self.tracker.resume_valid(&self.game)
            || !self.training.resume_valid()
            || !self.count_practice.resume_valid()
            || !self.wager_practice.resume_valid()
            || !self.deviation_practice.resume_valid()
            || !self.drill_generator.resume_valid()
            || !self.save().valid()
            || !self.pending_rules.valid()
        {
            return false;
        }
        if self.selected >= 512
            || self.page >= 768
            || self.card_page >= 128
            || self.amount < 0
            || self.amount > i64::MAX / 4
            || self.wager_amount < 0
            || self.wager_amount > i64::MAX / 4
            || self.count_answer < -21632
            || self.count_answer > 21632
            || self.card_group > self.game.seat_count() + 4
            || self.result_hand >= 4
            || self.cut_index >= 17
            || self.current_token > self.decision_counter
            || self.assisted_token > self.decision_counter
            || self
                .count_practice
                .resume_last_token()
                .is_some_and(|n| n > self.count_token)
            || self
                .wager_practice
                .resume_last_token()
                .is_some_and(|n| n > self.wager_token)
            || self.deviation_practice.resume_last_token() > self.drill_token
            || self
                .training
                .resume_last_token()
                .is_some_and(|n| n > self.decision_counter)
        {
            return false;
        }
        let selected_limit = match self.view {
            View::Menu => MENU.len() + self.extras().1,
            View::Wager | View::WagerPractice | View::WagerPolicy => 6,
            View::Amount | View::ProfileAmount => 5,
            View::Training => 11,
            View::Settings => 7,
            View::Sound => 9,
            // Display cursor3 stays Return; cursor4 is the new Card back row.
            // Hardware capability
            // is intentionally outside the private wire identity.
            View::Display => 5,
            View::AudioVolume | View::Brightness => 1,
            View::Profile | View::ProfileDepth => 8,
            View::ProfilePreset | View::CountPractice => 7,
            View::CoachCheck | View::Leave | View::Summary => 2,
            View::Session => 5,
            View::WagerAnswer | View::DeviationPractice => 9,
            View::WagerResult => 5,
            View::DeviationTarget => 32,
            _ => 512,
        };
        if self.selected >= selected_limit {
            return false;
        }
        if self.view == View::TrainingStats && self.page > 4
            || self.view == View::CountStats && self.page > 3
            || self.view == View::Cards && self.card_group > self.game.hand_count
        {
            return false;
        }
        if self.pending.as_ref().is_some_and(|d| !d.resume_valid())
            || self.live_ev.is_some_and(|v| !v.resume_valid())
            || self
                .reveal
                .is_some_and(|r| r.progress > 4 + self.game.extra_seats || r.bankroll < 0)
        {
            return false;
        }
        if self.other_reveal_len > 4
            || self.other_reveal_len == 0 && self.other_reveal_at != 0
            || self.other_reveal_len > 0 && self.other_reveal_at >= self.other_reveal_len
        {
            return false;
        }
        for (i, entry) in self.other_reveals.iter().enumerate() {
            if i < self.other_reveal_len as usize {
                let Some(r) = entry else { return false };
                if r.seat > self.game.extra_seats
                    || r.hand_index >= 4
                    || !r.hand.resume_valid()
                    || r.hand.count() == 0
                {
                    return false;
                }
                let last = r.hand.count() - 1;
                let position = r.hand.positions()[last] as usize;
                if self.tracker.resume_seen(r.hand.positions()[last])
                    || self.game.resume_exposed_card(position) != Some(r.hand.cards()[last])
                {
                    return false;
                }
            } else if entry.is_some() {
                return false;
            }
        }
        true
    }
    fn rebase_checkpoint_clocks(&mut self, now: u32) {
        let shift = now.wrapping_sub(self.now);
        self.clock = self.clock.wrapping_add(shift);
        self.reveal_clock = self.reveal_clock.wrapping_add(shift);
        self.other_reveal_clock = self.other_reveal_clock.wrapping_add(shift);
        self.toast_clock = self.toast_clock.wrapping_add(shift);
        self.review_due = self.review_due.map(|n| n.wrapping_add(shift));
        self.now = now;
        if self.reveal.is_some() {
            self.reveal_clock = now
        }
        if self.other_reveal_len != 0 {
            self.other_reveal_clock = now
        }
        if self.view == View::Table
            && matches!(self.game.phase, Phase::Dealer | Phase::OtherPlayers)
        {
            self.clock = now
        }
        if self.view == View::Burn {
            self.clock = now
        }
        if self.view == View::Table && self.game.phase == Phase::Result && self.review_due.is_some()
        {
            self.review_due = Some(now)
        }
    }
    pub(crate) fn resume_rebase(&mut self, now: u32) {
        self.rebase_checkpoint_clocks(now);
        self.controls = Controls::EMPTY;
        self.sounds = SoundQueue::EMPTY;
        self.load_allowed = false;
        self.dirty = true;
    }
    /// Exclude persistence/presentation latency from visible-face/review dwell while retaining held-key and queued-cue state.
    pub fn checkpoint_output_clock(&mut self, now: u32) {
        self.rebase_checkpoint_clocks(now);
    }
}
impl App {
    /// Storage adapters render the shared chooser without observing live cards or applying assistance.
    pub fn resume_commit_clock(&mut self, now: u32) {
        self.resume_rebase(now);
        self.render();
    }
    pub fn render_resume_chooser(&mut self, chooser: &crate::resume_ui::BootChooser) {
        use crate::resume_ui::BootState;
        let mut w = crate::widgets::Widgets {
            frame: &mut self.frame,
            renderer: &mut self.renderer,
        };
        let subtitle = if matches!(chooser.state, BootState::ReadFailure { .. }) {
            "Saved session could not be read"
        } else {
            "Saved privately on this device"
        };
        w.title("RESUME SESSION", Some(subtitle), false);
        let row = match chooser.state {
            BootState::ChooseSaved { row } | BootState::ReadFailure { row } => row as usize,
            _ => 0,
        };
        w.list(&chooser.rows(), row, None);
        self.dirty = false;
    }
    pub fn render_resume_storage_error(&mut self) {
        self.render_resume_storage_error_choice(0);
    }
    pub fn render_resume_storage_error_choice(&mut self, row: u8) {
        let mut w = crate::widgets::Widgets {
            frame: &mut self.frame,
            renderer: &mut self.renderer,
        };
        w.title("SAVE RETRY", Some("Keeping your current session"), false);
        w.list(
            &["Retry saving session", "Start fresh without saving"],
            (row as usize).min(1),
            None,
        );
        self.dirty = false;
    }
    pub fn render_resume_read_error_choice(&mut self, row: u8) {
        let mut w = crate::widgets::Widgets {
            frame: &mut self.frame,
            renderer: &mut self.renderer,
        };
        w.title("READ RETRY", Some("Saved scores unavailable"), false);
        w.list(
            &["Retry reading saved scores", "Start fresh without saving"],
            (row as usize).min(1),
            None,
        );
        self.dirty = false;
    }
    pub fn render_unsaved_banner(&mut self) {
        use crate::{
            render::{Align, TextStyle},
            widgets::{DARK, GOLD},
        };
        self.frame.rect(181, 108, 59, 12, DARK);
        self.renderer.text(
            &mut self.frame,
            (210, 109),
            "UNSAVED",
            TextStyle {
                size: 9,
                bold: true,
                color: GOLD,
                align: Align::Center,
            },
        );
    }
}
#[cfg(feature = "validation")]
impl App {
    /// Fixed public diagnostic cases only. Requires a separately owned EMPTY benchmark App.
    #[doc(hidden)]
    pub fn prepare_fixed_benchmark(&mut self, case: u8, mode: Mode) -> bool {
        let Some(&seed) = [334, 205, 37, 19, 86, 165].get(case as usize) else {
            return false;
        };
        if self.game.shoe_number != 0
            || !self.boot(seed)
            || !self.game.cut(156)
            || !self.game.deal()
        {
            return false;
        }
        let valid = match case {
            0 => {
                self.game.phase == Phase::Player
                    && self.game.hands[0].cards().iter().all(|c| c.value() == 8)
            }
            1 => {
                self.game.phase == Phase::Player
                    && self.game.hands[0].cards().iter().all(|c| c.value() == 11)
            }
            2 => self.game.phase == Phase::Player && self.game.hands[0].soft(),
            3 => self.game.phase == Phase::Insurance,
            4 => {
                self.game.phase == Phase::Player
                    && self.game.hands[0].cards().iter().all(|c| c.value() == 2)
            }
            5 => {
                self.game.phase == Phase::Player
                    && self.game.hands[0].cards().iter().all(|c| c.value() == 3)
            }
            _ => false,
        };
        if !valid || !self.game.invariant() {
            return false;
        }
        self.training.mode = mode;
        self.view = View::Table;
        self.round_id = 1;
        self.test_round = mode == Mode::Test;
        self.reveal = None;
        self.observe();
        self.sounds = SoundQueue::EMPTY;
        self.dirty = false;
        true
    }
    #[doc(hidden)]
    pub fn fixed_benchmark_event(&mut self) {
        self.event(Event::Tap(if self.game.phase == Phase::Insurance {
            Key::Blue
        } else {
            Key::Grey
        }));
    }
}
#[cfg(test)]
mod resume_full_tests {
    use super::*;
    use crate::{cards::Card, deviation_drills as dd, resume_private as r, wager_practice as wp};
    fn packet(a: &App) -> std::vec::Vec<u8> {
        let mut b = std::vec![0;r::MAX_BYTES];
        let n = r::capture_private(a, &mut b, 99, true).unwrap();
        b.truncate(n);
        b
    }
    #[test]
    fn all_populated_rings_pending_models_and_analytics_fit_and_continue() {
        let mut a = std::boxed::Box::new(App::new(7));
        a.resume_enabled = true;
        a.training.mode = Mode::Test;
        let mut hand = crate::cards::Hand::EMPTY;
        hand.bet = 500;
        hand.add(Card::from_id(3).unwrap(), 10);
        hand.add(Card::from_id(5).unwrap(), 11);
        let count = crate::deviations::Count {
            running: 3,
            undealt: 104,
            decks: 6,
            observed: 208,
            shoe: 1,
        };
        let values = training::ExpectedValues {
            model: training::EvModel::InfiniteReplacementV1,
            original_wager: 500,
            legal_mask: 3,
            evaluated_mask: 3,
            best_mask: 2,
            initial_pair_funded: false,
            values: [-0.2, 0.1, f64::NAN, f64::NAN, f64::NAN],
        };
        for token in 1..=200u64 {
            let decision = a
                .training
                .record(training::Attempt {
                    token,
                    round: token as u32,
                    hand_index: 0,
                    profile: crate::rules::Profile::SixH17,
                    hand,
                    dealer_up: Card::from_id(4).unwrap(),
                    legal_mask: 3,
                    choice: Choice::Play(Action::Hit),
                    policy: Policy::Basic,
                    count: None,
                    assisted: token % 2 == 0,
                    values: Some(values),
                })
                .decision
                .unwrap();
            assert!(a.session_metrics.record_card(decision));
            let snapshot = Snapshot {
                running: 3,
                undealt: 104,
                decks: 6,
                observed: 208,
                shoe: 1,
                session: 1,
            };
            assert!(a.count_practice.begin(
                Question {
                    token,
                    kind: Kind::TrueCount,
                    given_half_decks: Some(4)
                },
                snapshot
            ));
            a.count_practice
                .submit(token, Answer::TrueCount(1), token % 2 == 0);
            let ws = wp::Snapshot {
                context: wp::Context {
                    session: 1,
                    checkpoint: token,
                    shoe: 1,
                },
                rules: crate::rules::Rules::default(),
                count,
                bankroll: 100000,
                predeal: true,
                estimated_raw_tc: Some(1.5),
            };
            assert!(a.wager_practice.begin(token, ws, wp::Policy::default()));
            let wa = a
                .wager_practice
                .submit(token, wp::Choice::Wager(1000), token % 2 == 0)
                .attempt
                .unwrap();
            assert!(a.session_metrics.record_wager(wa));
            let q = dd::generate(
                token,
                dd::IndexId::Hard16v10,
                crate::rules::Profile::SixNoSurrender,
                dd::Boundary::At,
            )
            .unwrap();
            assert!(a.deviation_practice.begin(q));
            a.deviation_practice
                .submit(token, Choice::Play(Action::Hit), token % 2 == 0);
            let round = crate::session_analysis::Round {
                id: token,
                base_wager: 500,
                stake: 500,
                returned: 500,
                bankroll: 100000,
                deposits: 100000,
                cashouts: 0,
            };
            assert!(a.session_analysis.record(round));
        }
        a.decision_counter = 200;
        a.current_token = 200;
        a.count_token = 201;
        a.wager_token = 201;
        a.drill_token = 201;
        assert!(a.count_practice.begin(
            Question {
                token: 201,
                kind: Kind::TrueCount,
                given_half_decks: Some(4)
            },
            Snapshot {
                running: 3,
                undealt: 104,
                decks: 6,
                observed: 208,
                shoe: 1,
                session: 1
            }
        ));
        a.count_practice.request_help(201);
        let mut ws = a.wager_practice.newest(0).unwrap().reference.snapshot;
        ws.context.checkpoint = 201;
        assert!(a.wager_practice.begin(201, ws, wp::Policy::default()));
        a.wager_practice.request_help(201);
        let q = a.deviation_practice.replay_missed(199, 201).unwrap();
        assert!(a.deviation_practice.begin(q));
        a.deviation_practice.request_help(201);
        a.render();
        let p = packet(&a);
        let public = a.save().encode().unwrap();
        let mut bound = std::vec![0;r::MAX_BYTES];
        let n = crate::resume_binding::bind_private(
            &mut bound,
            crate::resume_binding::PublicCommit {
                generation: 7,
                checkpoint_epoch: 99,
                transaction_session: [17; 16],
                packet: &public,
            },
            &p,
        )
        .unwrap();
        std::println!(
            "full_all_rings_private_bytes={} bound_bytes={n} App_bytes={}",
            p.len(),
            core::mem::size_of::<App>()
        );
        assert!(n < 262000);
        let mut frozen = std::boxed::Box::new(App::EMPTY);
        a.capture_clone_into(&mut frozen).unwrap();
        assert_eq!(packet(&frozen), p);
        let mut restored = std::boxed::Box::new(App::EMPTY);
        r::restore_candidate(&mut restored, &p, 0, true).unwrap();
        assert_eq!(packet(&restored), p);
        assert_eq!(
            restored.session_analysis.summary(),
            a.session_analysis.summary()
        );
        assert_eq!(restored.session_metrics.costs(), a.session_metrics.costs());
        assert!(restored.count_practice.reference(201).is_some());
        assert!(restored.wager_practice.reference(201).is_some());
        assert!(restored.deviation_practice.reference(201).is_some());
        a.deviation_practice
            .submit(201, Choice::Play(Action::Stand), false);
        restored
            .deviation_practice
            .submit(201, Choice::Play(Action::Stand), false);
        assert_eq!(packet(&a), packet(&restored));
    }
}
#[cfg(test)]
mod seated_resume_tests {
    use super::*;
    use crate::resume_private as r;
    fn packet(a: &App) -> std::vec::Vec<u8> {
        let mut b = std::vec![0;r::MAX_BYTES];
        let n = r::capture_private(a, &mut b, 42, true).unwrap_or_else(|e| {
            panic!(
                "capture {e:?} phase{:?} seat{} queue{}/{}",
                a.game.phase, a.game.player_seat, a.other_reveal_at, a.other_reveal_len
            )
        });
        b.truncate(n);
        b
    }
    #[test]
    fn all_seats_pending_visibility_and_emergency_pools_continue() {
        let mut queues = 0;
        let mut phases = 0u32;
        let mut samples = 0;
        for extra in 0..=6 {
            for player in 0..=extra {
                for seed in [1, 7, 99] {
                    let mut a = std::boxed::Box::new(App::new(seed));
                    assert!(a.game.configure_seats(extra, player));
                    let mut now = 0u32;
                    for _ in 0..120 {
                        a.render();
                        a.checkpoint_output_clock(now);
                        let p = packet(&a);
                        let mut b = std::boxed::Box::new(App::EMPTY);
                        r::restore_candidate(&mut b, &p, now, true).unwrap();
                        assert!(
                            packet(&b) == p,
                            "restored bytes differ extra{extra} player{player} seed{seed} phase{:?}",
                            a.game.phase
                        );
                        assert_eq!(a.frame.pixels, b.frame.pixels);
                        queues += usize::from(a.other_reveal_len != 0 || a.reveal.is_some());
                        phases |= 1 << a.game.phase as u8;
                        samples += 1;
                        now = now.wrapping_add(801);
                        for app in [&mut *a, &mut *b] {
                            app.tick(now);
                            if app.game.phase == Phase::Player
                                && !app.is_busy()
                                && app.game.legal(Action::Split)
                            {
                                app.perform(Choice::Play(Action::Split));
                            } else {
                                app.event(Event::Tap(
                                    if app.game.phase == Phase::Player
                                        && app.game.hands[app.game.active].total() < 16
                                    {
                                        Key::Grey
                                    } else {
                                        Key::Blue
                                    },
                                ));
                            }
                            app.render();
                        }
                        assert!(
                            packet(&a) == packet(&b),
                            "next step differs extra{extra} player{player} seed{seed}"
                        );
                    }
                }
            }
        }
        std::println!(
            "seated_continuations={samples} pending_visibility={queues} phases={phases:#x}"
        );
        assert!(queues > 100);
        assert_ne!(phases & (1 << Phase::OtherPlayers as u8), 0);
    }
    #[test]
    fn revision_tracks_events_auto_stages_and_new_aid_but_not_idle_time() {
        let mut a = App::new(7);
        let n = a.checkpoint_revision();
        a.tick(20);
        a.render();
        assert_eq!(n, a.checkpoint_revision());
        a.event(Event::Tap(Key::Blue));
        assert!(a.checkpoint_revision() > n);
        a.event(Event::Tap(Key::Blue));
        a.render();
        let n = a.checkpoint_revision();
        a.tick(300);
        assert!(a.checkpoint_revision() > n);
        let n = a.checkpoint_revision();
        a.render();
        a.render();
        assert_eq!(n, a.checkpoint_revision());
        a.mark_aid(1);
        let n = a.checkpoint_revision();
        a.mark_aid(1);
        assert_eq!(n, a.checkpoint_revision());
        a.mark_aid(4);
        assert!(a.checkpoint_revision() > n);
    }
}
#[cfg(test)]
mod output_dwell_tests {
    use super::*;
    #[test]
    fn long_output_retry_preserves_held_controls_cues_and_face_review_dwell() {
        let mut a = App::new(7);
        a.fast_mode = false;
        a.event(Event::Tap(Key::Blue));
        a.event(Event::Tap(Key::Blue));
        assert!(a.reveal.is_some());
        let progress = a.reveal.unwrap().progress;
        a.controls.update([true, false], 100, false);
        assert!(
            a.controls
                .update([true, false], 950, false)
                .into_iter()
                .flatten()
                .any(|e| e == Event::Hold(Key::Blue))
        );
        a.emit(Cue::Loss);
        a.checkpoint_output_clock(10000);
        assert!(
            a.controls
                .update([true, false], 10001, false)
                .into_iter()
                .flatten()
                .next()
                .is_none()
        );
        let mut loss = false;
        while let Some(c) = a.pop_sound() {
            loss |= c == Cue::Loss
        }
        assert!(loss);
        a.tick(10179);
        assert_eq!(a.reveal.unwrap().progress, progress);
        a.tick(10180);
        assert_eq!(a.reveal.unwrap().progress, progress + 1);
        a.reveal = None;
        a.game.phase = Phase::Result;
        a.view = View::Table;
        a.training.mode = Mode::Test;
        a.test_round = true;
        a.round_id = 5;
        a.last_review = 4;
        a.review_due = Some(10000);
        a.now = 10890;
        a.checkpoint_output_clock(20000);
        a.tick(20899);
        assert_eq!(a.view, View::Table);
        a.tick(20900);
        assert_eq!(a.view, View::RoundReview);
    }
}
#[cfg(all(test, feature = "validation"))]
mod fixed_benchmark_tests {
    use super::*;
    #[test]
    fn fixed_separate_benchmark_cases_cover_modes_without_live_app_changes() {
        let live = App::new(99);
        let before = live.save();
        for mode in [Mode::Guide, Mode::Test, Mode::Coach] {
            for case in 0..6 {
                let mut b = std::boxed::Box::new(App::EMPTY);
                assert!(b.prepare_fixed_benchmark(case, mode));
                assert_eq!(b.game.rules.minimum, 500);
                if case < 2 {
                    assert!(b.game.legal(Action::Split));
                }
                b.fixed_benchmark_event();
                assert_eq!(b.training.stats().all.decisions, 1);
            }
        }
        assert_eq!(live.save(), before);
    }
}
