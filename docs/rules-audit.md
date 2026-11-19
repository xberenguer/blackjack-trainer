# Rules and table-condition audit

These are explicit generic training profiles, described as Vegas-style only. They do not represent a verified current table at a named casino. The default `$5–$500` limits are trainer settings, not a claim about current casino minimums or maximums.

## Implemented fixed conditions

| Condition | Simulator behavior |
|---|---|
| Decks | Six, except the named eight-deck profile. No one/two-deck profile is implemented. |
| Default dealer play | Hit soft 17. The S17 profile stands on every 17. All profiles stand on hard 17–21 and draw below 17. |
| Natural payout | Default 3:2; the separately labeled 6:5 profile pays 6:5. A split 21 is ordinary 21 and pays 1:1 when it wins. |
| Deal/peek | Face-up player cards; one dealer upcard and one face-down hole card. Insurance is offered against an ace before peek. Dealer blackjack is resolved before player hit/double/split/surrender decisions; the ten-upcard peek is automatic. |
| Double | Any initial two cards; exactly one additional card. Double after split is allowed except in the no-DAS profile. No doubling split aces. Full equal additional wager is required and must be affordable. |
| Split | Two cards with equal blackjack value, including different ten-value ranks. At most four hands per seat, with funded equal wagers. Non-ace pairs can resplit within that cap. |
| Split aces | Exactly one additional card per split ace; no hitting, doubling or resplitting aces. This no-RSA condition applies to every current profile. |
| Surrender | Late surrender on an unsplit initial two-card hand, after dealer-blackjack exclusion. Return half the wager. Disabled in the no-surrender profile; no early surrender. |
| Insurance | Optional against an ace, for exactly half the original main wager when affordable; 2:1 net payout on dealer blackjack. No side bets. |
| Betting | Exact cents internally, `$5` main-wager increments. Defaults `$5` minimum / `$500` maximum. Valid custom bounds are multiples of `$5`, at least `$5`, with maximum no greater than `$10,000`; current UI bounds must be disclosed separately. Main wagers freeze before dealing. |
| Seats | Default one controlled spot. Optional zero to six simulated tablemates and a selectable controlled position. Tablemates use rule-specific basic strategy, flat table-minimum virtual wagers, separate chip ledgers and automatic virtual refills; they decline insurance. They do not use the count or hidden cards. |
| Order | One first card to each seated player, dealer upcard, each player's second card, then dealer hole card. Player decisions proceed from dealer-left seat to dealer-right; dealer play follows all player decisions. |
| Player cut | Rotates the shuffled stack; valid insertion positions leave at least one deck on each side. It does not change the dealer penetration marker. |
| Burn | One unseen burn after the cut and after an emergency discard-rack shuffle. Burn and unrevealed dealer-hole ranks never enter the count. |
| Penetration | Default 75%. The engine accepts explicit bounds through 90%; intended settings add 80/85/90%. A round already in progress completes after crossing the marker, then all cards are shuffled before the next round. |
| Discards/exhaustion | Busts, surrender and paid naturals leave the layout; other hands stay held until settlement. If the active shoe runs out, only the discard rack is shuffled/cut/burned and the round continues. Held cards never enter that pool. A full shuffle is forced after that round. If there is no safe discard-and-burn pool, no held cards are recycled and play pauses explicitly. |
| Count epoch | Recovery advances the count epoch. Previously mixed discards become unknown; currently held exposed faces remain known, and the unrevealed hole stays unknown. The divisor is the actual remaining available shoe, not the original full-shoe cursor. |

## Source verification and scope

The chosen dealing and recovery procedure is a named **NJ-style simulation choice**, not a universal Las Vegas procedure. The official [New Jersey Chapter 69F rules](https://www.nj.gov/oag/ge/docs/Regulations/CHAPTER69F.pdf), sections 13:69F-2.6(e), (j) and (n), support table order, the hole-card/peek variant and completing a round after the cut card. Section 13:69F-2.15(f) describes shuffling the discard rack, burning and completing an exhausted round. This simulator additionally makes its next-round full-shuffle choice explicit. No claim is made that every casino uses this irregularity procedure.

The [Venetian/Palazzo gaming guide](https://www.venetianlasvegas.com/content/dam/vlvweb/casino/table-games/VP-Gaming-Guide.pdf), PDF page 5 (zero-based page 4), describes two-, six- and eight-deck games, 3:2/6:5 payouts, one-card split aces, and surrender only on six-deck shoes. Its no-resplitting-aces statement is specific to double deck. It does not verify that this trainer's blanket no-RSA shoe profiles match a current Venetian shoe table. The guide also does not establish all the fixed split caps, dealer soft-17 choices, penetration and current limits needed for a complete one-to-one preset.

[MGM's blackjack guide](https://www.mgmresorts.com/en/gamesense/guide-to-blackjack.html) identifies minimum/maximum limits and natural payouts as posted table conditions. Therefore trainer limits are shown as editable practice parameters; they are not hardcoded casino-price claims.

Sources were checked on 2026-11-18. A named casino preset requires a separately verified complete combination of deck count, H17/S17, payout, DAS, double restrictions, surrender, resplit cap/RSA, ace restrictions, hole-card/peek method and current posted limits. Missing conditions must remain labeled unknown. Adding RSA or a different dealing model requires new engine and split-EV validation rather than relabeling the current behavior.
