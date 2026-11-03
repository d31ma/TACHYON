use super::{
    AppFeature, AppTarget, Choice, ProjectChoices, RealmChoice, ReplyParser, TacLanguage,
    YonLanguage,
};
use crate::Failure;
use crate::failure::diagnostic;
use std::io::{BufRead, Write};

pub struct ChoicePrompt<'a> {
    input: &'a mut dyn BufRead,
    output: &'a mut dyn Write,
    number: usize,
}

impl<'a> ChoicePrompt<'a> {
    const INTRODUCTION: &'static str = "Answer a few questions to create your app. Press Enter to keep the answer that is shown.\n";
    const ONE: &'static str = "Type one number.";
    const MANY: &'static str = "Type numbers, separated by commas.";
    const TRY_AGAIN: &'static str = "Please type only numbers from the list.\n";
    const NONE: &'static str = "none";
    const IO_CODE: u16 = 1408;

    pub fn new(input: &'a mut dyn BufRead, output: &'a mut dyn Write) -> Self {
        Self {
            input,
            output,
            number: 0,
        }
    }

    pub fn ask(&mut self) -> Result<ProjectChoices, Failure> {
        self.say(Self::INTRODUCTION)?;
        let realm = self.ask_one(RealmChoice::ALL, RealmChoice::Both)?;
        let choices = ProjectChoices {
            realm,
            tac: self.ask_tac(realm)?,
            yon: self.ask_yon(realm)?,
            targets: self.ask_targets(realm)?,
            features: self.ask_features(realm)?,
        };
        choices.checked()
    }

    fn ask_tac(&mut self, realm: RealmChoice) -> Result<Vec<TacLanguage>, Failure> {
        if !realm.has_client() {
            return Ok(Vec::new());
        }
        self.ask_many(TacLanguage::ALL, &[TacLanguage::JavaScript])
    }

    fn ask_yon(&mut self, realm: RealmChoice) -> Result<Vec<YonLanguage>, Failure> {
        if !realm.has_server() {
            return Ok(Vec::new());
        }
        self.ask_many(YonLanguage::ALL, &[YonLanguage::JavaScript])
    }

    fn ask_targets(&mut self, realm: RealmChoice) -> Result<Vec<AppTarget>, Failure> {
        if !realm.has_client() {
            return Ok(Vec::new());
        }
        self.ask_many(AppTarget::ALL, &[AppTarget::Web])
    }

    fn ask_features(&mut self, realm: RealmChoice) -> Result<Vec<AppFeature>, Failure> {
        let options = Self::features_for(realm);
        self.ask_many(&options, &[])
    }

    fn features_for(realm: RealmChoice) -> Vec<AppFeature> {
        if realm.has_server() {
            return AppFeature::ALL.to_vec();
        }
        AppFeature::ALL
            .iter()
            .copied()
            .filter(|feature| !feature.needs_server())
            .collect()
    }

    fn ask_one<T: Choice>(&mut self, options: &[T], default: T) -> Result<T, Failure> {
        self.show_question(options)?;
        let hint = Self::hint(Self::ONE, &Self::numbers(options, &[default]));
        self.read_valid(&hint, &|reply| ReplyParser::one(reply, options, default))
    }

    fn ask_many<T: Choice>(&mut self, options: &[T], defaults: &[T]) -> Result<Vec<T>, Failure> {
        self.show_question(options)?;
        let hint = Self::hint(Self::MANY, &Self::numbers(options, defaults));
        self.read_valid(&hint, &|reply| ReplyParser::many(reply, options, defaults))
    }

    fn show_question<T: Choice>(&mut self, options: &[T]) -> Result<(), Failure> {
        self.number += 1;
        let lines: Vec<String> = options
            .iter()
            .enumerate()
            .map(|(index, option)| format!("  {}. {}\n", index + 1, option.label()))
            .collect();
        let text = format!("\n{}. {}\n{}", self.number, T::QUESTION, lines.concat());
        self.say(&text)
    }

    fn read_valid<T>(
        &mut self,
        hint: &str,
        parse: &dyn Fn(&str) -> Option<T>,
    ) -> Result<T, Failure> {
        let mut answer = None;
        while answer.is_none() {
            answer = self.read_once(hint, parse)?;
        }
        answer.ok_or_else(|| Self::failure("The answer could not be read."))
    }

    fn read_once<T>(
        &mut self,
        hint: &str,
        parse: &dyn Fn(&str) -> Option<T>,
    ) -> Result<Option<T>, Failure> {
        self.say(hint)?;
        let reply = self.read_line()?;
        let answer = parse(&reply);
        if answer.is_none() {
            self.say(Self::TRY_AGAIN)?;
        }
        Ok(answer)
    }

    fn read_line(&mut self) -> Result<String, Failure> {
        let mut line = String::new();
        self.input
            .read_line(&mut line)
            .map_err(|error| Self::failure(&format!("Cannot read the answer: {error}")))?;
        Ok(line)
    }

    fn say(&mut self, text: &str) -> Result<(), Failure> {
        self.output
            .write_all(text.as_bytes())
            .and_then(|()| self.output.flush())
            .map_err(|error| Self::failure(&format!("Cannot show the question: {error}")))
    }

    fn hint(instruction: &str, numbers: &str) -> String {
        format!("{instruction} Press Enter for {numbers}: ")
    }

    fn numbers<T: Choice>(options: &[T], chosen: &[T]) -> String {
        let numbers: Vec<String> = options
            .iter()
            .enumerate()
            .filter(|(_, option)| chosen.contains(option))
            .map(|(index, _)| (index + 1).to_string())
            .collect();
        Self::joined_or_none(&numbers)
    }

    fn joined_or_none(numbers: &[String]) -> String {
        if numbers.is_empty() {
            return String::from(Self::NONE);
        }
        numbers.join(",")
    }

    fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(Self::IO_CODE, message, None, None))
    }
}
