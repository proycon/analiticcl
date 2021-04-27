use sesdiff::{EditScript,EditInstruction};

use crate::types::*;
use crate::anahash::*;

#[derive(Debug)]
pub struct Confusable {
    pub editscript: EditScript<String>,
    pub weight: f64,
}

impl Confusable {
    ///See if the confusable is found in a larger edit script
    pub fn found_in(&self, refscript: &EditScript<&str>) -> bool {
        let l = self.editscript.instructions.len();
        let mut matches = 0; //number of matching instructions
        let mut index = 0;
        for (i, refinstruction) in refscript.instructions.iter().enumerate() {
            if let Some(instruction) = self.editscript.instructions.get(matches) {
                let foundinstruction = match (instruction, refinstruction) {
                    (EditInstruction::Insertion(s), EditInstruction::Insertion(sref)) |  (EditInstruction::Deletion(s), EditInstruction::Deletion(sref)) => {
                        s == sref
                    },
                    (EditInstruction::Identity(s), EditInstruction::Identity(sref)) => {
                        if i == 0 && i == l -1 {
                            s == sref
                        } else if i == 0 {
                            sref.ends_with(s)
                        } else if i == l - 1 {
                            sref.starts_with(s)
                        } else {
                            s == sref
                        }
                    },
                    (EditInstruction::IdentityOptions(v), EditInstruction::Identity(sref)) => {
                        let mut foundoption = false;
                        for s in v.iter() {
                            if i == 0 && i == l -1 {
                                if s == sref { foundoption = true; break; }
                            } else if i == 0 {
                                if sref.ends_with(s) { foundoption = true; break; }
                            } else if i == l - 1 {
                                if sref.starts_with(s) { foundoption = true; break; }
                            } else {
                                if s == sref { foundoption = true; break; }
                            }
                        }
                        foundoption
                    }
                    (EditInstruction::InsertionOptions(v), EditInstruction::Insertion(sref)) | (EditInstruction::DeletionOptions(v), EditInstruction::Deletion(sref)) => {
                        let mut foundoption = false;
                        for s in v.iter() {
                            if s == sref {
                                foundoption = true;
                                break;
                            }
                        }
                        foundoption
                    },
                    _ => false
                };
                if !foundinstruction {
                    matches = 0;
                    continue; //try again with new reference offset
                } else {
                    matches += 1;
                    if matches == l {
                        return true;
                    }
                }
            }
        }
        false
    }
}


