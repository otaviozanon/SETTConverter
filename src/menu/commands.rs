// Command strings registered for each menu entry.

pub(super) fn command(exe: &str, target: &str) -> String {
    if target == "extract" {
        return format!("\"{exe}\" extract \"%1\"");
    }
    if target == "compress" {
        return format!("\"{exe}\" convert --to pdf \"%1\"");
    }
    if target == "cover" {
        return format!("\"{exe}\" convert --to jpg --cover \"%1\"");
    }
    if let Some(every) = target.strip_prefix("pdf-split:") {
        return format!("\"{exe}\" convert --to pdf --split-every {every} \"%1\"");
    }
    if let Some(degrees) = target.strip_prefix("rotate:") {
        return format!("\"{exe}\" convert --to pdf --rotate {degrees} \"%1\"");
    }
    if let Some(format) = target.strip_prefix("long:") {
        return format!("\"{exe}\" convert --to {format} --long \"%1\"");
    }
    if let Some(format) = target.strip_prefix("sheet:") {
        return format!("\"{exe}\" convert --to {format} --sheet \"%1\"");
    }
    if let Some(format) = target.strip_prefix("frames:") {
        return format!("\"{exe}\" convert --to {format} --frames \"%1\"");
    }
    format!("\"{exe}\" convert --to {target} \"%1\"")
}

/// Folder entry: same options, but `%V` when started from the background.
pub(super) fn group_command(exe: &str, target: &str, placeholder: &str) -> String {
    if target == "pdf-merge" {
        return format!("\"{exe}\" convert --to pdf --merge \"{placeholder}\"");
    }
    format!("\"{exe}\" convert --to {target} \"{placeholder}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_encode_every_target_kind() {
        let exe = r"C:\tools\SETTConverter.exe";
        assert_eq!(
            command(exe, "jpg"),
            r#""C:\tools\SETTConverter.exe" convert --to jpg "%1""#
        );
        assert_eq!(
            command(exe, "pdf-split:10"),
            r#""C:\tools\SETTConverter.exe" convert --to pdf --split-every 10 "%1""#
        );
        assert_eq!(
            command(exe, "frames:png"),
            r#""C:\tools\SETTConverter.exe" convert --to png --frames "%1""#
        );
        assert_eq!(
            command(exe, "rotate:90"),
            r#""C:\tools\SETTConverter.exe" convert --to pdf --rotate 90 "%1""#
        );
        assert_eq!(
            command(exe, "long:png"),
            r#""C:\tools\SETTConverter.exe" convert --to png --long "%1""#
        );
        assert_eq!(
            command(exe, "sheet:jpg"),
            r#""C:\tools\SETTConverter.exe" convert --to jpg --sheet "%1""#
        );
        assert_eq!(
            command(exe, "cover"),
            r#""C:\tools\SETTConverter.exe" convert --to jpg --cover "%1""#
        );
        assert_eq!(
            command(exe, "compress"),
            r#""C:\tools\SETTConverter.exe" convert --to pdf "%1""#
        );
        assert_eq!(
            command(exe, "extract"),
            r#""C:\tools\SETTConverter.exe" extract "%1""#
        );
    }

    #[test]
    fn folder_commands_use_the_placeholder() {
        let exe = r"C:\tools\SETTConverter.exe";
        assert_eq!(
            group_command(exe, "zip", "%V"),
            r#""C:\tools\SETTConverter.exe" convert --to zip "%V""#
        );
        assert_eq!(
            group_command(exe, "pdf-merge", "%V"),
            r#""C:\tools\SETTConverter.exe" convert --to pdf --merge "%V""#
        );
    }
}
