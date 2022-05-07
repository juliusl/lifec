use carddeck::Dealer;
use lifec::Runtime;

fn main() {
    let mut runtime = Runtime::<Dealer>::default();
    runtime
        .on("{ setup;; }")
        .update(|s, _| (s.clone(), "{ deal;; }"))
        .test("()", "{ deal;; }");

    runtime
        .on("{ deal;; }")
        .dispatch("[+26][+26]", "{ action_choose; player_1; }");

    runtime
        .on("{ action_choose; player_1; }")
        .dispatch("[.0-1]", "{ action_choose; player_2; }");

    runtime
        .on("{ action_choose; player_2; }")
        .dispatch("[.1-1]", "{ after_choose;; }");

    runtime.on("{ after_choose;; }").update(|s, _| {
        let s = s.prune();
        if s.hand(1).is_none() {
            return (s.clone(), { "{ exit;; }" });
        }

        let deck = s.deck();
        let deck = deck.unwrap();
        let deck = &deck.take(2);
        if let Some((remaining,hands)) = deck {
            if remaining.cards().len() != 0 {
                return (s.clone(), "{ error;; }");
            } else if hands.len() != 2 {
                return (s.clone(), "{ error;; }");
            }

            let cards = hands;
            let p1 = &cards[0];
            let p2 = &cards[1];

            if p1 > p2 {
                (s.clone(), "{ after_choose; player_1; }")
            } else {
                (s.clone(), "{ after_choose; player_2; }")
            }
        } else {
            (s.clone(), "{ exit;; }")
        }
    })
    .test("[s2s3s4][s5s6s7](hah2)", "{ after_choose; player_1; }")
    .test("[s2s3s4][s5s6s7](h2ha)", "{ after_choose; player_2; }");

    runtime
        .on("{ after_choose; player_1; }")
        .dispatch("[.0+2]", "{ action_choose; player_1; }");
    runtime
        .on("{ after_choose; player_2; }")
        .dispatch("[.1+2]", "{ action_choose; player_1; }");

     runtime.test().start("{ setup;; }");
}
