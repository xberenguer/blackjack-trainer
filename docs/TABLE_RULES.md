# Supported table conditions

These are explicit generic Vegas-style training profiles. They are not advertised as a verified copy of a named casino's current table. The default $5 minimum and $500 maximum are training values; both are configurable in $5 steps. The starting bankroll is $1,000.

## Supported Rust 0.10 rules

| Condition | Supported behavior |
|---|---|
| Decks | Six; eight in the named eight-deck profile |
| Dealer | H17 by default; separate S17 profile |
| Natural | 3:2 by default; separate 6:5 profile |
| Double | Any initial two cards; one final card; full additional wager |
| Double after split | Enabled by default; separate no-DAS profile |
| Split | Equal blackjack values; up to four player hands; sequential play |
| Split aces | One card each; no resplitting or hitting split aces |
| Surrender | Late, initial unsplit two-card hand; separate no-surrender profile |
| Dealer hole card | US hole-card game; blackjack checked before ordinary player actions |
| Insurance | Optional half of original wager against an ace, 2:1 profit on dealer blackjack |
| Split 21 | Ordinary 21, not a natural blackjack |
| Shoe | Finite physical cards, unbiased shuffle, movable player cut, unseen burn |
| Dealer cut | Default 75%; menu choices 50/60/70/75/80/85/90%; complete the active round before a normal reshuffle |
| Betting | Original main-wager minimum/maximum enforced; $5 steps; authorized doubles/splits add their own stakes |

The player cut rotates the shuffled stack. It does not choose the dealer's penetration. The remaining physical shoe, rather than the number of observed faces, determines the displayed true-count divisor. Hidden burn and hole faces are excluded from the running count until actually exposed.

The six profiles combine the stated conditions only. Settings do not silently enable unsupported rules. Single/double deck, resplit aces, restricted-total doubling, double-for-less, early surrender, European no-hole-card, continuous shufflers, side bets and proprietary variants are not represented by the current profiles.

## Primary source comparison

[MGM's own blackjack guide](https://www.mgmresorts.com/en/gamesense/guide-to-blackjack.html) describes dealer hole cards, optional surrender, soft-17 variations and posted table minimum/maximum limits. It does not establish one complete rule set or current dollar limits for every MGM Las Vegas table.

[Venetian's gaming guide](https://www.venetianlasvegas.com/content/dam/vlvweb/casino/table-games/VP-Gaming-Guide.pdf), blackjack section, describes two/six/eight-deck games, both natural payouts, one-card split aces, and surrender on six-deck games. Its ace-resplit distinction does not match this trainer's fixed no-RSA shoe rule. A Venetian label would therefore overstate the current simulation's match. The guide also leaves some exact table conditions unspecified.

Zero to six other virtual seats are optional. They use basic strategy and flat minimum wagers with independent chip ledgers. Initial and terminal draw faces are shown before subsequent decisions or count quizzes. Zero extras keeps the ordinary single-player table.

If an exceptional deep-shoe round runs out of undealt cards, this implementation uses a specifically documented discard-only shuffle, unseen burn and new count epoch. Held cards remain excluded, the active round completes, and a full shuffle follows. This is a named simulation procedure; it does not establish every Las Vegas casino's exceptional procedure. See [engine validation and sources](extra-seats-engine.md).
