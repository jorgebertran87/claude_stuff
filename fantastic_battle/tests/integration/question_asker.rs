use cucumber::{then, when, World};

use fantastic_battle::container;
use fantastic_battle::domain::model::{Player, Question, Theme};
use fantastic_battle::domain::ports::QuestionAsker;

#[derive(Debug, Default, World)]
pub struct QuestionAskerWorld {
    asker: Option<Box<dyn QuestionAsker>>,
    first_question: Option<Question>,
    second_question: Option<Question>,
    current_question: Option<Question>,
}

impl QuestionAskerWorld {
    fn ensure_asker(&mut self) {
        if self.asker.is_none() {
            self.asker = Some(container::test_question_asker());
        }
    }
}

#[when(regex = r#"^the game asks for a question from "([^"]*)" for theme "([^"]*)"$"#)]
fn when_ask_question(world: &mut QuestionAskerWorld, npc_name: String, theme_name: String) {
    world.ensure_asker();
    let asker = world.asker.as_ref().unwrap();
    let theme = Theme::new(&theme_name).unwrap();
    let player = Player::new(&npc_name).unwrap();
    let question = asker.ask(&theme, &player);
    if world.first_question.is_none() {
        world.first_question = Some(question);
    } else {
        world.second_question = Some(question);
    }
    world.current_question = Some(asker.ask(&theme, &player));
}

#[then(regex = r#"^the question text is "([^"]*)"$"#)]
fn then_question_text(world: &mut QuestionAskerWorld, text: String) {
    let question = world.current_question.as_ref().expect("no question asked");
    assert_eq!(question.text(), &text);
}

#[then(regex = r#"^the correct answer is "([^"]*)"$"#)]
fn then_correct_answer(world: &mut QuestionAskerWorld, answer: String) {
    let question = world.current_question.as_ref().expect("no question asked");
    assert!(question.is_correct(&answer), "expected '{}' to be correct", answer);
}

#[then("the questions are different")]
fn then_questions_different(world: &mut QuestionAskerWorld) {
    let first = world.first_question.as_ref().expect("no first question");
    let second = world.second_question.as_ref().expect("no second question");
    assert_ne!(first.text(), second.text());
}

fn main() {
    futures::executor::block_on(QuestionAskerWorld::run(
        "features/question_asker.feature",
    ));
}
