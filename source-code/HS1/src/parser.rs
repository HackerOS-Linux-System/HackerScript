use pest::Parser;

#[derive(pest_derive::Parser)]
#[grammar = "hackerscript.pest"] // ten sam plik .pest co w HS3
pub struct HackerScriptParser;
