// https://stackoverflow.com/a/38406885
pub(crate) fn to_title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub(crate) fn count_chars_and_cases(string: &str) -> (usize, usize, usize) {
    string.chars().fold((0, 0, 0), |(total, lower, upper), c| {
        let is_lower = c.is_lowercase();
        let is_upper = c.is_uppercase();

        (
            total + 1,
            lower + usize::from(is_lower),
            upper + usize::from(is_upper),
        )
    })
}

#[doc(hidden)] // pub for bench
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MimicAction {
    Title,
    Uppercase,
    Nothing,
}

/// Allows examining string case when provided with info about characters
#[doc(hidden)] // pub for bench
pub trait LiteralString {
    fn chars(&self) -> (usize, bool, bool);

    /// Examine given string and tell which action to take to match it's case
    #[must_use]
    fn mimic_case_action(&self, from: &str) -> MimicAction {
        let (self_char_count, self_has_lowercase, self_has_uppercase) = self.chars();

        // do nothing if current string is:
        // - has at least one uppercase letter
        // - has no letters
        if self_has_uppercase || !self_has_lowercase {
            return MimicAction::Nothing;
        }

        let (char_count, lowercase, uppercase) = count_chars_and_cases(from);

        // uppercase: has no lowercase letters and at least one uppercase letter
        if (lowercase == 0 && uppercase != 0)
            // either current string is 1 letter or string is upper and is long
            && (self_char_count == 1 || char_count > 1)
        {
            return MimicAction::Uppercase;
        }

        // there is exactly one uppercase letter
        if uppercase == 1
            // either one letter long or first letter is upper
            && (char_count == 1 || from.chars().next().is_some_and(char::is_uppercase))
        {
            return MimicAction::Title;
        }

        MimicAction::Nothing
    }
}

// replaces a SINGLE REQUIRED "{}" template in string. braces can be escaped by doubling "{{" "}}"
pub(crate) fn runtime_format_single_value(template: &str, value: &str) -> Result<String, String> {
    let mut result = String::new();

    let mut formatted = false;
    let mut previous = None;

    for (i, c) in template.chars().enumerate() {
        match c {
            '{' => {
                if let Some('{') = previous {
                    result.push('{');
                    previous = None;
                } else {
                    previous = Some('{');
                }
            }
            '}' => match (previous, formatted) {
                (Some('{'), true) => return Err(format!("unmatched '{{' at position {i}")),
                (Some('{'), false) => {
                    result.push_str(value);
                    formatted = true;
                    previous = None;
                }
                (Some('}'), _) => {
                    result.push('}');
                    previous = None;
                }
                (None, _) => previous = Some('}'),
                (Some(_), _) => unreachable!(),
            },
            _ => {
                if let Some(previous) = previous {
                    return Err(format!("unmatched '{previous}' at position {i}"));
                }

                result.push(c);
            }
        }
    }

    if !formatted {
        return Err("string did not contain {} template".to_owned());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_format_formats() {
        assert_eq!(runtime_format_single_value("{}", "1").unwrap(), "1");
        assert_eq!(runtime_format_single_value(" {}", "2").unwrap(), " 2");
        assert_eq!(runtime_format_single_value("{} ", "3").unwrap(), "3 ");
    }

    #[test]
    fn runtime_format_escapes() {
        assert_eq!(
            runtime_format_single_value("}} {{{}}}", "1").unwrap(),
            "} {1}"
        );
    }

    #[test]
    fn runtime_format_requires_replacement() {
        assert!(runtime_format_single_value("hello {{", "world").is_err());
    }

    #[test]
    fn runtime_format_one_replacement() {
        assert!(runtime_format_single_value("hello {} {}", "world").is_err());
    }

    #[test]
    fn runtime_format_unmatched() {
        assert!(runtime_format_single_value("}0", "world").is_err());
        assert!(runtime_format_single_value("{0", "world").is_err());
    }
}
