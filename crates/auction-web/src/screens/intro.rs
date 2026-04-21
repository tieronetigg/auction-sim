use auction_core::types::AuctionType;
use yew::prelude::*;

use crate::app::Screen;

#[derive(Properties, PartialEq)]
pub struct IntroScreenProps {
    pub auction_type: AuctionType,
    pub on_navigate: Callback<Screen>,
}

/// Static content for the theory intro — name, CSS type key, and body paragraphs.
struct IntroContent {
    name: &'static str,
    dot_type: &'static str,
    tagline: &'static str,
    body: &'static [&'static str],
    key_fact: &'static str,
    params: &'static [(&'static str, &'static str)],
}

fn content_for(auction_type: AuctionType) -> IntroContent {
    match auction_type {
        AuctionType::English => IntroContent {
            name: "English Auction",
            dot_type: "english",
            tagline: "Open ascending price",
            body: &[
                "The auctioneer calls out a starting price and bidders openly raise it. \
                 The auction closes after a period of silence — the standing high bidder \
                 wins and pays the price at which bidding stopped.",
                "The dominant strategy under the independent private values (IPV) model \
                 is straightforward: stay in as long as the price is below your value, \
                 drop out the moment it crosses it. There is no benefit to bluffing \
                 or strategically dropping early.",
                "Revenue equivalence: under symmetric IPV, the English auction yields \
                 the same expected revenue as the Vickrey, Dutch, and First-Price \
                 Sealed-Bid mechanisms. The winner pays approximately the \
                 second-highest value in the room.",
            ],
            key_fact: "The equilibrium is fully efficient: the bidder with the highest \
                       value always wins. This is not true of all mechanisms.",
            params: &[
                ("Item",     "Vintage Chronograph Watch"),
                ("Your value", "$350"),
                ("AI bidders", "5  (Alice $420, Bob $380, Carol $310, Dave $450, Eve $290)"),
                ("Reserve",  "$300"),
                ("Timeout",  "25 s silence"),
            ],
        },
        AuctionType::Dutch => IntroContent {
            name: "Dutch Auction",
            dot_type: "dutch",
            tagline: "Descending clock: first caller wins",
            body: &[
                "The price starts high and falls at a fixed rate. The first bidder \
                 to press Call wins the item at the current clock price. No one \
                 else bids; the auction is over the instant someone acts.",
                "Strategic equivalence: a Dutch auction is strategically identical \
                 to a First-Price Sealed-Bid auction. The optimal call price is \
                 (n-1)/n × your value — the same Bayes-Nash equilibrium bid you \
                 would submit in FPSB. Waiting for a lower price increases your \
                 surplus if you win, but risks losing to a faster bidder.",
                "Unlike the English auction, you receive no information about \
                 other bidders' values while the clock falls. You must act on your \
                 prior beliefs alone.",
            ],
            key_fact: "Dutch auctions are widely used for perishable goods: flowers, \
                       fish, and treasury bills. The speed of resolution is the \
                       mechanism's main practical advantage.",
            params: &[
                ("Item",       "Vintage Chronograph Watch"),
                ("Your value", "$350"),
                ("Start price","$550"),
                ("Drop rate",  "$8 / second"),
                ("Floor",      "$50"),
            ],
        },
        AuctionType::FirstPriceSealedBid => IntroContent {
            name: "First-Price Sealed-Bid",
            dot_type: "fpsb",
            tagline: "Sealed bids: winner pays their own bid",
            body: &[
                "Every bidder submits a single sealed bid before the deadline. \
                 The highest bid wins and pays exactly what they bid. There is \
                 no opportunity to revise based on others' behaviour.",
                "Because you pay your own bid, bidding your true value guarantees \
                 zero surplus if you win. The optimal strategy is to shade your bid \
                 downward. Under symmetric IPV with n bidders and uniform values, \
                 the Bayes-Nash equilibrium bid is (n-1)/n × your value.",
                "With 6 bidders, the equilibrium shade is 5/6 ≈ 83% of your value. \
                 The AI bidders in this simulation bid at 80% — close to the \
                 theoretical optimum. Revenue equivalence predicts the seller earns \
                 the same as in an English or Vickrey auction in expectation.",
            ],
            key_fact: "Bidding truthfully is a dominated strategy in FPSB. \
                       Any shade below your value strictly improves expected surplus.",
            params: &[
                ("Item",       "Vintage Chronograph Watch"),
                ("Your value", "$350"),
                ("AI bidders", "5  (Alice/Carol/Dave shade 0.80×; Bob/Eve truthful)"),
                ("Deadline",   "30 s"),
                ("Reserve",    "$80"),
            ],
        },
        AuctionType::Vickrey => IntroContent {
            name: "Vickrey Auction",
            dot_type: "vickrey",
            tagline: "Sealed bids: winner pays the second-highest bid",
            body: &[
                "Every bidder submits a sealed bid. The highest bidder wins but \
                 pays only the second-highest bid. If only one bid exceeds the \
                 reserve, the winner pays the reserve price.",
                "Truth dominance: bidding your true value is a weakly dominant \
                 strategy. No matter what others bid, you cannot improve your \
                 outcome by misreporting. Overbidding risks paying more than your \
                 value; underbidding risks losing an auction you should have won.",
                "The Vickrey mechanism is the single-item special case of the more \
                 general VCG mechanism. It is efficient and incentive-compatible, \
                 but rarely used in practice — sellers distrust a rule that often \
                 results in a price well below the winner's bid.",
            ],
            key_fact: "Vickrey is the only standard single-item mechanism where \
                       truthful bidding is a dominant strategy — not just an equilibrium.",
            params: &[
                ("Item",       "Vintage Chronograph Watch"),
                ("Your value", "$350"),
                ("AI bidders", "5  (all bid truthfully)"),
                ("Deadline",   "30 s"),
                ("Reserve",    "$80"),
            ],
        },
        AuctionType::AllPay => IntroContent {
            name: "All-Pay Auction",
            dot_type: "allpay",
            tagline: "Everyone pays their bid, highest bid wins",
            body: &[
                "Every bidder submits a sealed bid and pays it regardless of \
                 whether they win. The highest bidder receives the item; all \
                 others simply lose their bid with nothing to show for it.",
                "The equilibrium bidding strategy is more aggressive shading \
                 than FPSB. With n bidders and values drawn uniformly from [0, H], \
                 the Bayes-Nash equilibrium bid is b(v) = (n-1)/n · v · (v/H)^(n-1). \
                 With 6 bidders and H = $500, a bidder with value $350 bids \
                 approximately $112.",
                "Revenue equivalence holds: despite the very different payment \
                 structure, the seller's expected revenue equals that of the English, \
                 Dutch, FPSB, and Vickrey auctions under symmetric IPV.",
            ],
            key_fact: "All-pay auctions model lobbying, litigation, R&D races, and \
                       electoral contests — any situation where effort is sunk \
                       regardless of outcome.",
            params: &[
                ("Item",       "Vintage Chronograph Watch"),
                ("Your value", "$350"),
                ("AI bidders", "5  (all use BNE formula, n=6, H=$500)"),
                ("Deadline",   "30 s"),
            ],
        },
        AuctionType::Double => IntroContent {
            name: "Double Auction  (k-DA)",
            dot_type: "double",
            tagline: "Two-sided market: buyers and sellers, uniform clearing price",
            body: &[
                "Buyers submit bids and sellers submit asks simultaneously, \
                 before a shared deadline. Orders are sorted — bids descending, \
                 asks ascending — and all pairs where the bid exceeds the ask \
                 are matched. Every matched participant trades at the same \
                 uniform clearing price.",
                "With k = 0.5, the clearing price is the midpoint of the last \
                 matching bid-ask pair (the marginal pair). This is the k-double \
                 auction rule introduced by Chatterjee and Samuelson (1983). \
                 Under truthful bidding, the clearing price falls in the \
                 competitive equilibrium range.",
                "Myerson-Satterthwaite: no budget-balanced mechanism can be \
                 simultaneously efficient and incentive-compatible in a bilateral \
                 trade setting. The double auction trades off these properties — \
                 it is approximately efficient but not fully incentive-compatible.",
            ],
            key_fact: "You are a buyer. Your order influences the clearing price \
                       only if you are the marginal buyer. All matched buyers pay \
                       the same price — not their individual bids.",
            params: &[
                ("Item",      "Research Report"),
                ("Your value","$100  (buyer)"),
                ("Buyers",    "4  (Human $100, Alice $120, Bob $110, Carol $90)"),
                ("Sellers",   "4  (Dave ask $35, Eve $60, Fiona $80, Grant $105)"),
                ("Deadline",  "45 s"),
                ("k",         "0.5  (midpoint clearing)"),
            ],
        },
        AuctionType::Combinatorial => IntroContent {
            name: "Combinatorial Auction (Pay-as-Bid)",
            dot_type: "combinatorial",
            tagline: "Bundle bids on multiple items — winner pays their own bid",
            body: &[
                "A combinatorial auction lets bidders submit bids on packages \
                 (bundles) of multiple items simultaneously. Bids use XOR semantics: \
                 at most one of your bids will be selected. You can bid on the North \
                 Wing alone, the South Wing alone, or both wings together.",
                "Complementarities: a bundle of complementary items is worth more \
                 than the sum of its parts. Combinatorial auctions let bidders express \
                 this directly, avoiding the exposure problem of having to win all \
                 items or none.",
                "Payment rule: each winner pays exactly their own submitted bid \
                 (pay-as-bid). The welfare-maximising allocation is found by brute force. \
                 This gives no incentive to bid truthfully — just like FPSB.",
            ],
            key_fact: "XOR semantics: at most one package bid per bidder is selected, \
                       and no two winning packages may share an item.",
            params: &[
                ("Items",      "North Wing (N) + South Wing (S)"),
                ("Your value", "$100 for {N,S} bundle"),
                ("AI bidders", "Alice {N}=$20, Bob {S}=$15, Carol {N,S}=$40"),
                ("Deadline",   "30 s"),
                ("Payment",    "Pay-as-bid"),
            ],
        },
        AuctionType::Vcg => IntroContent {
            name: "VCG Mechanism",
            dot_type: "vcg",
            tagline: "Package bids — each winner pays their externality",
            body: &[
                "VCG (Vickrey-Clarke-Groves) selects the welfare-maximising allocation \
                 just like a combinatorial auction, but computes payments differently. \
                 Each winner pays the externality they impose on others: \
                 p_i = W*_{-i} − (W* − v_i)",
                "Strategy-proofness: bidding your true value is a weakly dominant strategy. \
                 Deviating can only hurt you — understating your value may cause you to lose \
                 a profitable allocation; overstating may cause you to win an item whose \
                 VCG payment exceeds your true value.",
                "Budget deficit: VCG revenue may be less than the winner's bids, \
                 requiring an external subsidy. Individual rationality is guaranteed: \
                 each winner's payment is at most their stated bid.",
            ],
            key_fact: "VCG is the multi-item generalisation of the Vickrey mechanism. \
                       Your payment equals the benefit others would gain if you had not \
                       participated.",
            params: &[
                ("Items",      "North Wing (N) + South Wing (S)"),
                ("Your value", "$100 for {N,S} bundle"),
                ("AI bidders", "Alice {N}=$20, Bob {S}=$15, Carol {N,S}=$40"),
                ("Deadline",   "30 s"),
                ("Payment",    "VCG externality"),
            ],
        },
    }
}

#[function_component]
pub fn IntroScreen(props: &IntroScreenProps) -> Html {
    let c = content_for(props.auction_type);
    let auction_type = props.auction_type;

    let go_back = {
        let on_navigate = props.on_navigate.clone();
        Callback::from(move |_: MouseEvent| {
            on_navigate.emit(Screen::Menu);
        })
    };

    let go_start = {
        let on_navigate = props.on_navigate.clone();
        Callback::from(move |_: MouseEvent| {
            let screen = match auction_type {
                AuctionType::Combinatorial | AuctionType::Vcg => {
                    Screen::CombinatorialAuction(auction_type)
                }
                _ => Screen::Auction(auction_type),
            };
            on_navigate.emit(screen);
        })
    };

    html! {
        <div class="page">
            <div class="content">
                <header class="intro-header">
                    <button class="intro-back" onclick={go_back}>
                        { "← Back" }
                    </button>
                    <p class="intro-mechanism-label" data-type={c.dot_type}>
                        { c.dot_type.to_uppercase() }
                    </p>
                    <h1 class="intro-title">{ c.name }</h1>
                    <p class="intro-tagline">{ c.tagline }</p>
                    <hr class="intro-rule" />
                </header>

                <div class="intro-body">
                    { for c.body.iter().map(|para| html! { <p>{ *para }</p> }) }

                    if !c.key_fact.is_empty() {
                        <div class="intro-keyfact">
                            <p class="intro-keyfact-label">{ "Key result" }</p>
                            <p>{ c.key_fact }</p>
                        </div>
                    }
                </div>

                if !c.params.is_empty() {
                    <div class="intro-params">
                        <p class="intro-params-label">{ "This simulation" }</p>
                        <table>
                            { for c.params.iter().map(|(label, value)| html! {
                                <tr>
                                    <td>{ *label }</td>
                                    <td>{ *value }</td>
                                </tr>
                            }) }
                        </table>
                    </div>
                }

                <div class="intro-actions">
                    <button class="btn-start" onclick={go_start}>
                        { "Start auction →" }
                    </button>
                </div>
            </div>
        </div>
    }
}
