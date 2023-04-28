use anyhow::bail;

// Create a function chunk_str_vec_into_max_size that takes 3 parameters. The first parameter, 'input'
// is a vector of strings. The second parameter, sep, is a separator. The third parameter, 'size' is
// the maximum size to create. The function should return a vector of strings, where each element in
// the result vector is as many elements from the input vector as possible, without going over the
// size limit. For example, given an input of ["abc", "def", "ghi", "jkl", "mno"] and a limit of 7,
// return ["abc def", "ghi jkl", "mno"].
// Errors when a chunk len()>size.
pub fn chunk_str_vec_into_max_size(
    mut input: Vec<String>,
    sep: &str,
    size: usize,
) -> anyhow::Result<Vec<String>> {
    input.reverse();
    let mut result = Vec::new();
    let mut current = String::new();
    while let Some(next) = input.pop() {
        if next.len() > size {
            bail!("Chunk of length {} too large for size {}", next.len(), size);
        }
        if current.len() + next.len() + sep.len() > size {
            result.push(current);
            current = next;
        } else {
            if !current.is_empty() {
                current.push_str(sep);
            }
            current.push_str(&next);
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
    fn chunk_str_vec_into_max_size_works() {
        let result = chunk_str_vec_into_max_size(
            vec![
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

        let result = chunk_str_vec_into_max_size(
            ('A'..='Z').map(|l| l.to_string()).collect::<Vec<_>>(),
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
    fn chunk_str_vec_into_max_size_has_overflow() {
        println!(
            "{:?}",
            chunk_str_vec_into_max_size(vec!["ABCDEF".to_string()], " ", 5)
        );
        assert!(matches!(
            chunk_str_vec_into_max_size(vec!["ABCDEF".to_string()], " ", 5),
            Err(_)
        ));
    }
}
