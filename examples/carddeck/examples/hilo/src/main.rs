use atlier::system::start_editor;
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
        .dispatch("[+26][+26]", "{ draw;; }");

    runtime
        .on("{ draw;; }")
        .dispatch("[.0-1][.1-1]", "{ after_draw;; }");

    runtime
        .on("{ after_draw;; }")
        .update(|s, _| {
            println!("Current Dealer: {}", s);

            let deck = s.deck();
            let deck = deck.unwrap();
            let deck = &deck.take(2);
            if let Some((remaining, hands)) = deck {
                if remaining.cards().len() != 0 {
                    return (s.clone(), "{ error;; }");
                }

                let cards = hands;
                let p1 = &cards[0];
                let p2 = &cards[1];
                println!("{} > {}", p1, p2);

                if p1 > p2 {
                    println!("P1 Wins\n");
                    (s.clone(), "{ after_choose; player_1; }")
                } else {
                    println!("P2 Wins\n");
                    (s.clone(), "{ after_choose; player_2; }")
                }
            } else {
                println!("Game Over\n");
                (s.clone(), "{ exit;; }")
            }
        })
        .test("[s2s3s4][s5s6s7](hah2)", "{ after_choose; player_1; }")
        .test("[s2s3s4][s5s6s7](h2ha)", "{ after_choose; player_2; }");

    runtime
        .on("{ after_choose; player_1; }")
        .dispatch("[.0+2]", "{ game_over;; }");
    runtime
        .on("{ after_choose; player_2; }")
        .dispatch("[.1+2]", "{ game_over;; }");

    runtime.on("{ game_over;; }").update(|s, _| {
        if s.prune().hand(1).is_none() {
            println!("Game Over\n");
            (s.clone(), { "{ exit;; }" })
        } else {
            (s.clone(), "{ draw;; }")
        }
    });

    start_editor::<Runtime<Dealer>>(
        "hilo",
        1920.0,
        1080.0,
        runtime.parse_event("{ setup;; }"),
        |ui, s| {
            use imgui::ChildWindow;

            let mut state = s.clone();
            ChildWindow::new("hilo").size([-1.0, -1.0]).build(ui, || {
                if ui.button("Step") {
                    state = state.process();
                }
                ui.same_line();
                if ui.button("Reset") {
                    state.reset();
                    state = state.parse_event("{ setup;; }");
                }

                if let Some(current_state) = state.current() {
                    ui.label_text("event", state.context().to_string());
                    ui.label_text("current_state", format!("{}", current_state));

                    if let Some(hand_1) = current_state.hand(0) {
                        ui.label_text("hand 1", format!("{}", hand_1))
                    }

                    if let Some(hand_2) = current_state.hand(1) {
                        ui.label_text("hand 2", format!("{}", hand_2))
                    }

                    if let Some(deck) = current_state.deck() {
                        if deck.cards().len() > 1 && current_state.hands() > 1 {
                            ui.label_text("deck", format!("{}", deck));
                            let cards = deck.cards();

                            let player_1 = cards.get(0);
                            let player_2 = cards.get(1);

                            if let (Some(p1), Some(p2)) = (player_1, player_2) {
                                if p1 > p2 {
                                    ui.label_text("Winner", "player 1")
                                } else {
                                    ui.label_text("Winner", "player 2")
                                }
                            }
                        }
                    }
                }
            });

            Some(state.to_owned())
        },
    );

    // runtime
    //     .test()
    //     .expect("runtime did not pass all tests")
    //     .start("{ setup;; }");
}
