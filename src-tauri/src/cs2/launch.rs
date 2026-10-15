//! Launching CS2 through Steam.

use crate::error::{Error, Result};

/// Each arg must be a single `-flag` or `+command`/value token without quotes,
/// newlines or command separators, so nothing can be smuggled through Steam.
pub fn validate_args(args: &[String]) -> Result<()> {
    for (i, a) in args.iter().enumerate() {
        let bad = a.is_empty()
            || a.len() > 128
            || a.chars().any(|c| matches!(c, '"' | '\'' | '\n' | '\r' | ';' | '&' | '|' | '<' | '>' | '^' | '%' | '\0'));
        // Values following a +command (e.g. "+fps_max", "0") are allowed as bare tokens.
        let starts_ok = a.starts_with('-') || a.starts_with('+') || (i > 0 && !a.contains(char::is_whitespace));
        if bad || !starts_ok {
            return Err(Error::InvalidInput(format!("argumento de arranque inválido: {a}")));
        }
    }
    Ok(())
}

pub fn launch(args: &[String]) -> Result<()> {
    validate_args(args)?;
    #[cfg(windows)]
    {
        let steam = super::steam_path().ok_or_else(|| Error::Unavailable("Steam não encontrado".into()))?;
        std::process::Command::new(steam.join("steam.exe"))
            .arg("-applaunch")
            .arg(super::APP_ID)
            .args(args)
            .spawn()?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err(Error::Unsupported)
    }
}

/// Launch options worth setting in Steam (Properties → Launch options).
///
/// - `-novid`: skips the intro video; harmless.
/// - `-nojoy`: skips joystick subsystem init; saves a little memory/startup time.
/// - `+exec vektor`: guarantees Vektor's cfg loads even if autoexec is skipped.
///
/// Deliberately NOT included: `-high` (can starve audio/input threads),
/// `-threads N` (CS2 schedules its own threads; forcing a count is usually worse),
/// `-tickrate` (no effect on Valve matchmaking servers).
pub fn recommended_options() -> Result<String> {
    Ok("-novid -nojoy +exec vektor".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn args_validation() {
        assert!(validate_args(&s(&["-novid", "+fps_max", "0"])).is_ok());
        assert!(validate_args(&s(&["novid"])).is_err());
        assert!(validate_args(&s(&["-novid", "+say \"x\""])).is_err());
        assert!(validate_args(&s(&["+exec", "a;quit"])).is_err());
        assert!(validate_args(&s(&["-x", "a b"])).is_err());
    }

    #[test]
    fn recommended_is_valid() {
        let o = recommended_options().unwrap();
        let v: Vec<String> = o.split_whitespace().map(String::from).collect();
        assert!(validate_args(&v).is_ok());
    }
}
