use std::fmt::Write;

fn _create_cv_ss() -> String {
    let mut css = String::new();

    for i in 1..=20 {
        writeln!(
            &mut css,
            ".ss{i:02} {{ font-feature-settings: \"ss{i:02}\" }}"
        )
        .unwrap();
    }

    for i in 1..=99 {
        writeln!(
            &mut css,
            ".cv{i:02} {{ font-feature-settings: \"cv{i:02}\" }}"
        )
        .unwrap();
    }
    css
}
