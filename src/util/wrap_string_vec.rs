use anyhow::bail;

/// Join a vector of strings with a separator, wrapping every time we would overflow the provided
/// size.
pub fn wrap_string_vec(input: &Vec<String>, sep: &str, size: usize) -> anyhow::Result<Vec<String>> {
    let mut result = Vec::new();
    let mut current = String::new();
    for next in input {
        if next.len() > size {
            bail!("Chunk of length {} too large for size {}", next.len(), size);
        }
        if current.len() + next.len() + sep.len() > size {
            result.push(current);
            current = next.clone();
        } else {
            if !current.is_empty() {
                current.push_str(sep);
            }
            current.push_str(next);
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_string_vec_works() {
        let result = wrap_string_vec(
            &vec![
                "abc".to_string(),
                "def".to_string(),
                "ghi".to_string(),
                "jkl".to_string(),
                "mno".to_string(),
            ],
            " ",
            7,
        )
        .unwrap();
        assert_eq!(
            result,
            vec![
                "abc def".to_string(),
                "ghi jkl".to_string(),
                "mno".to_string(),
            ]
        );

        let result = wrap_string_vec(
            &('A'..='Z').map(|l| l.to_string()).collect::<Vec<_>>(),
            " ",
            10,
        )
        .unwrap();
        assert_eq!(
            result,
            vec![
                "A B C D E".to_string(),
                "F G H I J".to_string(),
                "K L M N O".to_string(),
                "P Q R S T".to_string(),
                "U V W X Y".to_string(),
                "Z".to_string()
            ]
        );
    }

    #[test]
    fn wrap_string_vec_has_overflow() {
        assert!(wrap_string_vec(&vec!["ABCDEF".to_string()], " ", 5).is_err());
    }

    #[test]
    fn wrap_string_vec_empty_input() {
        let result = wrap_string_vec(&Vec::new(), " ", 10).unwrap();
        assert_eq!(result, Vec::<String>::new());
    }
}
