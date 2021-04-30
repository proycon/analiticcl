use sesdiff::{EditScript,EditInstruction};
use std::str::FromStr;
use std::io::{Error,ErrorKind};

#[derive(Debug)]
pub struct Confusable {
    pub editscript: EditScript<String>,
    pub weight: f64,
    pub strictbegin: bool,
    pub strictend: bool,
}

impl Confusable {
    pub fn new(editscript: &str, weight: f64) -> Result<Confusable,std::io::Error> {
        let strictbegin = editscript.get(0..1).expect("Checking first character") == "^";
        let l = editscript.len();
        let strictend = editscript.get((l - 1)..).expect("Checking last character") == "$";
        Ok(Confusable {
            editscript: if strictbegin && strictend {
                match EditScript::from_str(&editscript[1..l-1]) {
                    Ok(editscript) => editscript,
                    Err(err) => return Err(Error::new(ErrorKind::Other, format!("{:?}",err)))
                }
            } else if strictbegin {
                match EditScript::from_str(&editscript[1..]) {
                    Ok(editscript) => editscript,
                    Err(err) => return Err(Error::new(ErrorKind::Other, format!("{:?}",err)))
                }
            } else if strictend {
                match EditScript::from_str(&editscript[..l-1]) {
                    Ok(editscript) => editscript,
                    Err(err) => return Err(Error::new(ErrorKind::Other, format!("{:?}",err)))
                }
            } else {
                match EditScript::from_str(editscript) {
                    Ok(editscript) => editscript,
                    Err(err) => return Err(Error::new(ErrorKind::Other, format!("{:?}",err)))
                }
            },
            weight: weight,
            strictbegin: strictbegin,
            strictend: strictend,
        })
    }

    ///See if the confusable is found in a larger edit script
    pub fn found_in(&self, refscript: &EditScript<&str>) -> bool {
        let l = self.editscript.instructions.len();
        let mut matches = 0; //number of matching instructions
        for (i, refinstruction) in refscript.instructions.iter().enumerate() {
            if let Some(instruction) = self.editscript.instructions.get(matches) {
                let foundinstruction = match (instruction, refinstruction) {
                    (EditInstruction::Insertion(s), EditInstruction::Insertion(sref)) |  (EditInstruction::Deletion(s), EditInstruction::Deletion(sref)) => {
                        sref.ends_with(s)
                    },
                    (EditInstruction::Identity(s), EditInstruction::Identity(sref)) => {
                        if matches == 0 && matches == l -1 {
                            s == sref
                        } else if matches == 0 {
                            sref.ends_with(s)
                        } else if matches == l - 1 {
                            sref.starts_with(s)
                        } else {
                            s == sref
                        }
                    },
                    (EditInstruction::InsertionOptions(v), EditInstruction::Insertion(sref)) | (EditInstruction::DeletionOptions(v), EditInstruction::Deletion(sref)) => {
                        let mut foundoption = false;
                        for s in v.iter() {
                            if sref.ends_with(s) { foundoption = true; break; }
                        }
                        foundoption
                    },
                    (EditInstruction::IdentityOptions(v), EditInstruction::Identity(sref)) => {
                        let mut foundoption = false;
                        for s in v.iter() {
                            if matches == 0 && matches == l -1 {
                                if s == sref { foundoption = true; break; }
                            } else if matches == 0 {
                                if sref.ends_with(s) { foundoption = true; break; }
                            } else if matches == l - 1 {
                                if sref.starts_with(s) { foundoption = true; break; }
                            } else {
                                if s == sref { foundoption = true; break; }
                            }
                        }
                        foundoption
                    },
                    _ => { false }
                };
                if !foundinstruction {
                    matches = 0;
                    if self.strictbegin {
                        return false;
                    } else {
                        continue; //try again with new reference offset
                    }
                } else {
                    matches += 1;
                    if matches == l {
                        if self.strictend {
                            return i == refscript.instructions.len() - 1;
                        } else {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}


