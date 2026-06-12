use serde::Deserialize;

/// Shop APIs serve prices either as decimal strings or JSON numbers,
/// sometimes both across endpoints of the same shop; accept both.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Price {
    Text(String),
    Number(f64),
}

impl Price {
    pub(crate) fn to_cents(&self) -> anyhow::Result<u64> {
        match self {
            Price::Number(euros) => Ok((euros * 100.0).round() as u64),
            Price::Text(text) => {
                let (euros, cents) = text.split_once('.').unwrap_or((text, "0"));
                let euros: u64 = euros.parse()?;
                let cents: u64 = format!("{cents:0<2}")[..2].parse()?;
                Ok(euros * 100 + cents)
            }
        }
    }
}
