use carddeck::Dealer;
use lifec::{Event, Runtime};

fn main() {
    let mut runtime = get_runtime();
    runtime
        .on("{ setup;; }")
        .call("setup")
        .test("()", "{ action_deal; _; _ }");

    runtime
        .on("{ action_deal;; }")
        .dispatch("[+26][+26]", "{ action_draw;; }");

    runtime
        .on("{ action_draw;; }")
        .dispatch("[.0-1][.1-1]", "{ after_draw;; }");

    runtime
        .on("{ after_draw;; }")
        .call("after_draw")
        .test("[s2s3s4][s5s6s7](hah2)", "{ after_choose; player_1; _ }")
        .test("[s2s3s4][s5s6s7](h2ha)", "{ after_choose; player_2; _ }");

    runtime
        .on("{ after_choose; player_1; }")
        .dispatch("[.0+2]", "{ game_over;; }");
    runtime
        .on("{ after_choose; player_2; }")
        .dispatch("[.1+2]", "{ game_over;; }");

    runtime
        .on("{ game_over;; }")
        .call("game_over");

    runtime
        .on("{ test_test;; }")
        .call("test_args").args(&[
            "--test",
            "value123",
            "--object",
            "'{test: abc, test123: 12345}'",
        ]);

    let runtime = runtime.parse_event("{ test_test;; }").process_state();

    lifec::editor::open_editor_with(
        format!("Dealer Editor"), 
        runtime, 
        vec![Dealer::dealer_section()],
        |_| {}, 
        |_| {},
        |_, _| {}
    )
}

fn get_runtime() -> Runtime<Dealer> {
    Runtime::<Dealer>::default()
        .with_call("setup", |s, _| (s.clone(), "{ deal;; }".to_string()))
        .with_call("after_draw", |s, _| {
            println!("Current Dealer: {}", s);

            let deck = s.deck();
            let deck = deck.unwrap();
            let deck = &deck.take(2);
            if let Some((remaining, hands)) = deck {
                if remaining.cards().len() != 0 {
                    return (s.clone(), "{ error;; }".to_string());
                }

                let cards = hands;
                let p1 = &cards[0];
                let p2 = &cards[1];
                println!("{} > {}", p1, p2);

                if p1 > p2 {
                    println!("P1 Wins\n");
                    (s.clone(), "{ after_choose; player_1; }".to_string())
                } else {
                    println!("P2 Wins\n");
                    (s.clone(), "{ after_choose; player_2; }".to_string())
                }
            } else {
                println!("Game Over\n");
                (s.clone(), "{ exit;; }".to_string())
            }
        })
        .with_call("game_over", |s, _| {
            if s.prune().hand(1).is_none() {
                println!("Game Over\n");
                (s.clone(), "{ exit;; }".to_string())
            } else {
                (s.clone(), "{ draw;; }".to_string())
            }
        })
        .with_call_args("test_args", |s, _| {
            let args = s.get_args();

            let map = s.parse_flags();

            println!("from test_args call: {:?}", args);
            println!("from test_args call: {:?}", map);

            (s.get_state(), Event::exit().to_string())
        })
}
