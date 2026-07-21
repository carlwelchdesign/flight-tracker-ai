#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SensitiveWriteRisk {
    PersonalContact,
    CredentialMaterial,
}

pub(super) fn classify_sensitive_write(content: &str) -> Option<SensitiveWriteRisk> {
    if contains_credential_material(content) {
        return Some(SensitiveWriteRisk::CredentialMaterial);
    }
    if contains_email_address(content) {
        return Some(SensitiveWriteRisk::PersonalContact);
    }
    None
}

fn contains_credential_material(content: &str) -> bool {
    let normalized = content.to_ascii_lowercase();
    if normalized.contains("-----begin ") && normalized.contains("private key-----") {
        return true;
    }
    if has_labeled_value(&normalized, "bearer", 16) {
        return true;
    }
    if [
        "api_key",
        "apikey",
        "access_token",
        "client_secret",
        "password",
        "secret",
    ]
    .iter()
    .any(|label| has_assignment(&normalized, label, 8))
    {
        return true;
    }
    normalized
        .split(|character: char| character.is_whitespace() || "\"'`()[]{}<>,;".contains(character))
        .any(|token| {
            (token.starts_with("ghp_") && token.len() >= 20)
                || (token.starts_with("github_pat_") && token.len() >= 24)
                || (token.starts_with("sk-") && token.len() >= 20)
        })
}

fn has_labeled_value(content: &str, label: &str, minimum_value_length: usize) -> bool {
    content.match_indices(label).any(|(index, _)| {
        if !has_label_boundary(content, index) {
            return false;
        }
        let value = content[index + label.len()..]
            .trim_start_matches(|character: char| character.is_whitespace() || character == ':');
        value
            .split(|character: char| {
                character.is_whitespace() || "\"'`()[]{}<>,;".contains(character)
            })
            .next()
            .is_some_and(|candidate| candidate.len() >= minimum_value_length)
    })
}

fn has_assignment(content: &str, label: &str, minimum_value_length: usize) -> bool {
    content.match_indices(label).any(|(index, _)| {
        if !has_label_boundary(content, index) {
            return false;
        }
        let suffix = &content[index + label.len()..];
        let suffix = suffix.trim_start();
        let Some(value) = suffix
            .strip_prefix('=')
            .or_else(|| suffix.strip_prefix(':'))
        else {
            return false;
        };
        value
            .trim_start()
            .split(|character: char| {
                character.is_whitespace() || "\"'`()[]{}<>,;".contains(character)
            })
            .next()
            .is_some_and(|candidate| candidate.len() >= minimum_value_length)
    })
}

fn has_label_boundary(content: &str, index: usize) -> bool {
    index == 0
        || content[..index]
            .chars()
            .next_back()
            .is_some_and(|character| !character.is_ascii_alphanumeric() && character != '_')
}

fn contains_email_address(content: &str) -> bool {
    content
        .split(|character: char| character.is_whitespace() || "\"'`()[]{}<>,;".contains(character))
        .map(|candidate| candidate.trim_matches(['.', ':']))
        .any(|candidate| {
            let Some((local, domain)) = candidate.split_once('@') else {
                return false;
            };
            !local.is_empty()
                && !domain.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && local.chars().all(|character| {
                    character.is_ascii_alphanumeric() || ".!#$%&'*+-/=?^_`{|}~".contains(character)
                })
                && domain
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || ".-".contains(character))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_credential_material_without_returning_matches() {
        for content in [
            "Authorization: Bearer test-token-value-123456",
            "api_key = test-secret-value",
            "github_pat_12345678901234567890",
            "-----BEGIN PRIVATE KEY----- fake-test-material",
        ] {
            assert_eq!(
                classify_sensitive_write(content),
                Some(SensitiveWriteRisk::CredentialMaterial)
            );
        }
    }

    #[test]
    fn detects_personal_email_addresses() {
        assert_eq!(
            classify_sensitive_write("Call operations.person@example.test before dispatch"),
            Some(SensitiveWriteRisk::PersonalContact)
        );
    }

    #[test]
    fn ignores_normal_operational_language_and_placeholders() {
        for content in [
            "Route around the marine layer and review fuel at the next waypoint",
            "Bearer reported as the shipment owner",
            "password: reset",
            "nosecret=ordinaryvalue",
            "Contact dispatch on the assigned radio frequency",
        ] {
            assert_eq!(classify_sensitive_write(content), None);
        }
    }
}
