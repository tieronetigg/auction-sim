use auction_core::types::AuctionType;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

// ── Body text ──────────────────────────────────────────────────────────────────

const ENGLISH_BODY: &str = "\
HOW IT WORKS

An English auction starts at a low opening price. Any participant may \
raise the standing high bid at any time. When no new bid arrives within \
a set silence period, the hammer falls and the highest bidder wins.

DOMINANT STRATEGY

Staying in the auction while the price is below your private value — \
and dropping out when the price reaches your value — weakly dominates \
all other strategies. Bidding above your value risks negative surplus; \
dropping out early forfeits obtainable gains.

REVENUE EQUIVALENCE

Under the standard independent private values model, an English auction \
produces the same expected revenue as a Vickrey (second-price sealed-bid) \
auction. In both formats, the winner is the highest-value bidder who pays \
approximately the second-highest value.

WINNER'S CURSE

In common-value settings (e.g., oil-field auctions where the oil is \
worth the same to everyone), winning signals that you were the most \
optimistic bidder — your estimate likely overstates true value. Rational \
bidders shade their bids downward to compensate. This simulation uses \
pure private values, so the curse does not apply here.

YOUR SETUP

You will compete against five AI bidders (Alice, Bob, Carol, Dave, Eve) \
with fixed private values. The AI bidders bid truthfully — they stay in \
until the price exceeds their value. Your private value is shown on the \
auction screen. You may bid freely, or simply observe.";

const DUTCH_BODY: &str = "\
HOW IT WORKS

A Dutch (descending-clock) auction starts at a high price that falls \
continuously. The first participant to call \"I'll take it!\" wins the \
item at that exact price. If the price reaches the floor without a \
caller, no sale occurs.

STRATEGIC EQUIVALENCE WITH FPSB

The Dutch auction is strategically equivalent to the First-Price \
Sealed-Bid (FPSB) auction. In both, you commit to a price before \
knowing your rivals' strategies, and you pay exactly what you bid. \
Calling early wins the item but sacrifices potential surplus; \
calling late risks losing to a rival who calls first.

OPTIMAL STRATEGY (SYMMETRIC IPV)

With n symmetric bidders drawing values uniformly, the equilibrium \
strategy is to call when the price reaches (n-1)/n of your true value. \
With six bidders that is roughly 5/6 ≈ 83% of your value. \
A bidder worth $300 should call around $250.

YOUR SETUP

You compete against five truthful AI bidders (Alice, Bob, Carol, Dave, Eve). \
The AI bidders call as soon as the price drops to their true value — they \
do not shade strategically. Your private value is shown on screen. \
Press Enter or Space to call the current price.";

const FPSB_BODY: &str = "\
HOW IT WORKS

In a First-Price Sealed-Bid (FPSB) auction, every participant submits \
a single secret bid. The highest bidder wins the item and pays exactly \
their own bid. No one sees anyone else's bid until after the deadline.

BID SHADING

Because you pay what you bid, bidding your true value yields zero \
surplus even if you win. Rational bidders shade downward: bid less \
than their value to capture a positive profit margin. How much to shade \
depends on the competition.

EQUILIBRIUM BID (SYMMETRIC IPV)

With n symmetric bidders and values drawn uniformly from [0, V], the \
Bayes-Nash equilibrium bid is (n-1)/n × value. With six bidders that \
is 5/6 ≈ 83% of your value. AI bidders in this simulation shade to \
approximately 75% of their value — slightly below equilibrium, so \
strategic play by you is rewarded.

YOUR SETUP

You have 30 seconds to enter and submit your single sealed bid. \
The AI bidders all submit early in the window. After the deadline, \
all bids are revealed and the winner is announced.";

const VICKREY_BODY: &str = "\
HOW IT WORKS

In a Vickrey (Second-Price Sealed-Bid) auction, every participant \
submits a single secret bid. The highest bidder wins the item but pays \
only the second-highest bid — not their own.

DOMINANT STRATEGY: TRUTH-TELLING

Bidding your true value is a weakly dominant strategy in Vickrey. \
• Bidding above value: risk winning and paying more than you gain \
  if the second-highest bid is also above your value. \
• Bidding below value: risk losing when you could have won profitably. \
In either case, deviating from truth-telling can only hurt you.

REVENUE EQUIVALENCE

A Vickrey auction generates the same expected revenue as an English \
auction. Both allocate to the highest-value bidder who effectively \
pays the second-highest value, despite their very different mechanics.

YOUR SETUP

You have 30 seconds to enter and submit your sealed bid. The AI bidders \
all bid their true values (the dominant strategy). After the deadline, \
all bids are revealed and the winner is announced. Try bidding your true \
value and observe — the outcome confirms the Vickrey truth-dominance result.";

const ALLPAY_BODY: &str = "\
HOW IT WORKS

An all-pay auction is like a sealed-bid auction with one key twist: \
every participant pays their submitted bid, win or lose. The highest \
bidder still wins the item, but all losing bidders forfeit their bids \
with nothing to show for it.

WHY IT MATTERS

All-pay auctions model real-world contests: lobbying expenditures, \
political campaigns, R&D races, and litigation. In each case, effort \
(the \"bid\") is sunk regardless of who wins.

EQUILIBRIUM BID SHADING

With n symmetric bidders and values drawn uniformly from [0, H], the \
unique symmetric Bayes-Nash equilibrium bid is:

    b(v) = (n-1)/n · v · (v/H)^(n-1)

With 6 bidders this yields bids far below true value — a bidder worth \
$350 bids only ~$49. This is not timidity; it reflects the high \
expected cost of the contest.

REVENUE EQUIVALENCE

Despite looking nothing like a Vickrey or English auction, the all-pay \
auction produces the same expected revenue under symmetric independent \
private values. The revenue equivalence theorem applies broadly.

YOUR SETUP

You have 30 seconds to submit a single sealed bid. All five AI bidders \
use the equilibrium formula above. After the deadline, all bids are \
revealed and the winner (highest bidder) receives the item — but \
everyone pays their bid.";

const COMBINATORIAL_BODY: &str = "\
HOW IT WORKS

A combinatorial auction lets bidders submit bids on packages (bundles) \
of multiple items simultaneously. You can bid $80 for item A alone, $70 \
for item B alone, or $200 for {A, B} together — these are XOR bids: at \
most one of your bids will be selected.

COMPLEMENTARITIES AND SUBSTITUTES

When items are complements, a bundle is worth more than the sum of its \
parts. Buying both wings of a building is worth more if you need \
contiguous space. Substitutes work the opposite way. Combinatorial \
auctions let bidders express these preferences directly, preventing the \
\"exposure problem\" of having to win all items or none.

WINNER DETERMINATION

The auctioneer finds the allocation that maximises total value across all \
submitted bids — subject to: (i) each item goes to at most one bidder, \
and (ii) at most one bid per bidder is selected (XOR semantics). This is \
an NP-hard optimisation problem in general; this simulator uses brute \
force over small instances.

PAYMENT RULE (PAY-AS-BID)

In this game, winners pay exactly their submitted bid. There is no \
incentive to bid truthfully — shading your bid trades off winning \
probability against profit margin, just like in a first-price sealed-bid \
auction.

YOUR SETUP

Two items: North Wing and South Wing of a property. Three AI bidders \
have fixed private values for individual wings or the full bundle. You \
have a private value for the bundle. Use Tab to cycle your package \
selection, then type your bid and press Enter. You have 30 seconds.";

const VCG_BODY: &str = "\
HOW IT WORKS

The VCG (Vickrey-Clarke-Groves) mechanism extends the Vickrey \
second-price idea to multi-item settings. As in a combinatorial auction, \
each bidder submits package bids and the welfare-maximising allocation \
is chosen. But payments are computed differently.

THE VCG PAYMENT FORMULA

Each winner i pays their externality: the harm their presence imposes \
on the other bidders:

    p_i  =  W*_{-i}  −  (W*  −  v_i)

where W* is total welfare with everyone, W*_{-i} is total welfare \
excluding bidder i, and v_i is bidder i's winning bid. In English: \
your payment equals what others would have gained if you had not \
participated, minus the value you added beyond your own bid.

STRATEGY-PROOFNESS

Bidding your true value is a weakly dominant strategy in VCG. \
Deviating can only harm you: understating causes you to lose \
allocations you should win; overstating can give you items whose VCG \
cost exceeds your true value.

INDIVIDUAL RATIONALITY

Every winner's VCG payment is at most their stated bid, so winners \
never regret participating.

BUDGET DEFICIT

VCG revenue may be less than the total welfare generated. The mechanism \
may need an external subsidy, unlike budget-balanced alternatives (e.g. \
a k-DA). This is a fundamental trade-off when combining efficiency with \
incentive-compatibility.

YOUR SETUP

Same scenario as the Combinatorial Auction: two wings, three AI bidders, \
30 seconds. The only difference is the payment rule — your VCG payment \
reflects the harm your participation imposes on the other bidders.";

const DOUBLE_BODY: &str = "\
HOW IT WORKS

A double auction brings buyers and sellers together in a single \
mechanism. Buyers submit bids (maximum willingness to pay) and sellers \
submit asks (minimum acceptable price). At the deadline all orders are \
revealed and a uniform clearing price is determined.

k-DOUBLE AUCTION (k = 0.5)

Orders are sorted: buyer bids descending, seller asks ascending. \
All pairs where bid >= ask are matched. The clearing price is the \
midpoint of the last matching bid and ask:

    p = (b_k + a_k) / 2

Every matched buyer pays p; every matched seller receives p. \
Unmatched participants do not trade.

BUDGET BALANCE

Total buyer payments equal total seller receipts — the mechanism is \
budget-balanced. No external subsidy is needed, unlike VCG.

INCENTIVE COMPATIBILITY

Unlike Vickrey, bidding your true value is NOT a dominant strategy in \
a k-DA. Your bid can influence the clearing price. However, with many \
participants the price impact is small and truthful bidding is a good \
approximation.

EFFICIENCY

A k-DA may fail to execute some mutually beneficial trades (those near \
the margin). This is an unavoidable trade-off: no budget-balanced \
mechanism can achieve full efficiency and incentive-compatibility \
simultaneously (Myerson-Satterthwaite impossibility theorem).

YOUR SETUP

You are a buyer with a private value. Three AI buyers and four AI \
sellers all bid/ask truthfully. You have 45 seconds to submit your bid.";

// ── State ──────────────────────────────────────────────────────────────────────

pub struct IntroState {
    pub auction_type: AuctionType,
    pub scroll: u16,
}

impl IntroState {
    pub fn new(auction_type: AuctionType) -> Self {
        IntroState { auction_type, scroll: 0 }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(3);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(3);
    }

    fn body_text(&self) -> &'static str {
        match self.auction_type {
            AuctionType::English => ENGLISH_BODY,
            AuctionType::Dutch => DUTCH_BODY,
            AuctionType::FirstPriceSealedBid => FPSB_BODY,
            AuctionType::Vickrey => VICKREY_BODY,
            AuctionType::AllPay => ALLPAY_BODY,
            AuctionType::Double => DOUBLE_BODY,
            AuctionType::Combinatorial => COMBINATORIAL_BODY,
            AuctionType::Vcg => VCG_BODY,
        }
    }

    fn title(&self) -> &'static str {
        match self.auction_type {
            AuctionType::English => " English Auction — How It Works ",
            AuctionType::Dutch => " Dutch Auction — How It Works ",
            AuctionType::FirstPriceSealedBid => " First-Price Sealed-Bid — How It Works ",
            AuctionType::Vickrey => " Vickrey Auction — How It Works ",
            AuctionType::AllPay => " All-Pay Auction — How It Works ",
            AuctionType::Double => " Double Auction (k-DA) — How It Works ",
            AuctionType::Combinatorial => " Combinatorial Auction — How It Works ",
            AuctionType::Vcg => " VCG Mechanism — How It Works ",
        }
    }

    fn border_color(&self) -> Color {
        match self.auction_type {
            AuctionType::English => Color::Cyan,
            AuctionType::Dutch => Color::Red,
            AuctionType::FirstPriceSealedBid => Color::Magenta,
            AuctionType::Vickrey => Color::Green,
            AuctionType::AllPay => Color::Yellow,
            AuctionType::Double => Color::Blue,
            AuctionType::Combinatorial => Color::LightGreen,
            AuctionType::Vcg => Color::LightMagenta,
        }
    }
}

// ── Render ─────────────────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, state: &IntroState) {
    let area = frame.size();

    let outer = Block::default()
        .title(state.title())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(state.border_color()));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let body = Paragraph::new(state.body_text())
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false })
        .scroll((state.scroll, 0));
    frame.render_widget(body, chunks[0]);

    let footer = Line::from(vec![
        Span::styled(
            "  ↑/↓  scroll     ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            "Space/Enter",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  start auction     ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled("  back", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(footer), chunks[1]);
}
