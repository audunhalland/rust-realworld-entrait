use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(sqlx::Type)]
pub struct Timestamptz(pub time::OffsetDateTime);

impl std::fmt::Display for Timestamptz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0
            .format_into(&mut IoFmtAdapter(f), &Rfc3339)
            .map_err(|_| std::fmt::Error::default())?;
        Ok(())
    }
}

// No fmt traits in time 0.3: https://github.com/time-rs/time/issues/375
struct IoFmtAdapter<'s, 'f>(&'s mut std::fmt::Formatter<'f>);

impl<'s, 'f> std::io::Write for IoFmtAdapter<'s, 'f> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let str = std::str::from_utf8(buf)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
        self.0
            .write_str(str)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Serialize for Timestamptz {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&self)
    }
}

impl<'de> Deserialize<'de> for Timestamptz {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StrVisitor;

        impl Visitor<'_> for StrVisitor {
            type Value = Timestamptz;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.pad("expected string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                OffsetDateTime::parse(v, &Rfc3339)
                    .map(Timestamptz)
                    .map_err(E::custom)
            }
        }

        deserializer.deserialize_str(StrVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamptz_display() {
        let ts = Timestamptz(OffsetDateTime::parse("2019-10-12T07:20:50.52Z", &Rfc3339).unwrap());
        assert_eq!("2019-10-12T07:20:50.52Z", format!("{}", ts));
    }
}
