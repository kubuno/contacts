use crate::models::contact::{AddressField, Contact, ContactField, CreateContactDto, DateField};

/// Export a single contact to vCard 3.0 format.
pub fn contact_to_vcard(c: &Contact) -> String {
    let mut v = String::new();
    v.push_str("BEGIN:VCARD\r\n");
    v.push_str("VERSION:3.0\r\n");
    v.push_str(&format!("UID:{}\r\n", c.vcard_uid));
    v.push_str(&format!("REV:{}\r\n", c.updated_at.format("%Y%m%dT%H%M%SZ")));

    // N field: family;given;middle;prefix;suffix
    v.push_str(&format!(
        "N:{};{};{};{};{}\r\n",
        c.family_name.as_deref().unwrap_or(""),
        c.given_name.as_deref().unwrap_or(""),
        c.middle_name.as_deref().unwrap_or(""),
        c.name_prefix.as_deref().unwrap_or(""),
        c.name_suffix.as_deref().unwrap_or(""),
    ));
    v.push_str(&format!("FN:{}\r\n", vcard_escape(&c.display_name)));

    if let Some(org) = &c.organization {
        if !org.is_empty() {
            let dept = c.department.as_deref().unwrap_or("");
            v.push_str(&format!("ORG:{};{}\r\n", vcard_escape(org), vcard_escape(dept)));
        }
    }
    if let Some(title) = &c.job_title {
        if !title.is_empty() {
            v.push_str(&format!("TITLE:{}\r\n", vcard_escape(title)));
        }
    }
    if let Some(nickname) = &c.nickname {
        if !nickname.is_empty() {
            v.push_str(&format!("NICKNAME:{}\r\n", vcard_escape(nickname)));
        }
    }

    for email in c.emails.0.iter() {
        let type_str = if email.field_type.is_empty() { "INTERNET".to_string() } else { email.field_type.to_uppercase() };
        v.push_str(&format!("EMAIL;TYPE={}:{}\r\n", type_str, vcard_escape(&email.value)));
    }
    for phone in c.phones.0.iter() {
        let type_str = if phone.field_type.is_empty() { "VOICE".to_string() } else { phone.field_type.to_uppercase() };
        v.push_str(&format!("TEL;TYPE={}:{}\r\n", type_str, vcard_escape(&phone.value)));
    }
    for addr in c.addresses.0.iter() {
        let type_str = if addr.field_type.is_empty() { "HOME".to_string() } else { addr.field_type.to_uppercase() };
        v.push_str(&format!(
            "ADR;TYPE={}:;;{};{};{};{};{}\r\n",
            type_str,
            vcard_escape(addr.street.as_deref().unwrap_or("")),
            vcard_escape(addr.city.as_deref().unwrap_or("")),
            vcard_escape(addr.region.as_deref().unwrap_or("")),
            vcard_escape(addr.postcode.as_deref().unwrap_or("")),
            vcard_escape(addr.country.as_deref().unwrap_or("")),
        ));
    }
    for url in c.urls.0.iter() {
        v.push_str(&format!("URL:{}\r\n", vcard_escape(&url.value)));
    }
    for date in c.dates.0.iter() {
        if date.field_type.to_lowercase() == "birthday" || date.field_type.to_lowercase() == "anniversaire" {
            v.push_str(&format!("BDAY:{}\r\n", date.value));
        } else {
            v.push_str(&format!("X-DATE;TYPE={}:{}\r\n", vcard_escape(&date.field_type), vcard_escape(&date.value)));
        }
    }
    if let Some(notes) = &c.notes {
        if !notes.is_empty() {
            v.push_str(&format!("NOTE:{}\r\n", vcard_escape(notes)));
        }
    }
    v.push_str("END:VCARD\r\n");
    v
}

pub fn contacts_to_vcf(contacts: &[Contact]) -> String {
    contacts.iter().map(contact_to_vcard).collect::<Vec<_>>().join("")
}

fn vcard_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace(',', "\\,")
     .replace(';', "\\;")
     .replace('\n', "\\n")
}

/// Parse a VCF string and return a list of CreateContactDto.
pub fn parse_vcf(vcf: &str) -> Vec<CreateContactDto> {
    let mut contacts = vec![];
    let mut in_card = false;
    let mut fields: Vec<(String, String, String)> = vec![]; // (name, params, value)
    let mut lines_iter = vcf.lines().peekable();

    while let Some(line) = lines_iter.next() {
        // Handle line folding (lines starting with space/tab continue the previous)
        let mut full_line = line.to_string();
        while let Some(next) = lines_iter.peek() {
            if next.starts_with(' ') || next.starts_with('\t') {
                full_line.push_str(next.trim_start());
                lines_iter.next();
            } else {
                break;
            }
        }

        let upper = full_line.to_uppercase();
        if upper == "BEGIN:VCARD" {
            in_card = true;
            fields.clear();
            continue;
        }
        if upper == "END:VCARD" && in_card {
            in_card = false;
            if let Some(dto) = fields_to_dto(&fields) {
                contacts.push(dto);
            }
            continue;
        }
        if !in_card { continue; }

        // Split name;params:value
        if let Some(colon_pos) = full_line.find(':') {
            let key_part = &full_line[..colon_pos];
            let value    = &full_line[colon_pos + 1..];
            if let Some(semi_pos) = key_part.find(';') {
                let name   = &key_part[..semi_pos];
                let params = &key_part[semi_pos + 1..];
                fields.push((name.to_uppercase(), params.to_uppercase(), value.to_string()));
            } else {
                fields.push((key_part.to_uppercase(), String::new(), value.to_string()));
            }
        }
    }

    contacts
}

fn fields_to_dto(fields: &[(String, String, String)]) -> Option<CreateContactDto> {
    let mut dto = CreateContactDto {
        given_name: None, middle_name: None, family_name: None,
        name_prefix: None, name_suffix: None, nickname: None,
        display_name: None, organization: None, department: None,
        job_title: None, avatar_color: None, pronouns: None,
        emails: vec![], phones: vec![], addresses: vec![],
        urls: vec![], dates: vec![], relations: vec![],
        instant_messages: vec![], custom_fields: vec![],
        notes: None, is_starred: None,
    };

    for (name, params, value) in fields {
        match name.as_str() {
            "N" => {
                let parts: Vec<&str> = value.splitn(5, ';').collect();
                dto.family_name  = parts.first().map(|s| vcard_unescape(s)).filter(|s| !s.is_empty());
                dto.given_name   = parts.get(1).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty());
                dto.middle_name  = parts.get(2).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty());
                dto.name_prefix  = parts.get(3).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty());
                dto.name_suffix  = parts.get(4).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty());
            }
            "FN" => {
                let fn_val = vcard_unescape(value);
                if !fn_val.is_empty() { dto.display_name = Some(fn_val); }
            }
            "ORG" => {
                let parts: Vec<&str> = value.splitn(2, ';').collect();
                dto.organization = parts.first().map(|s| vcard_unescape(s)).filter(|s| !s.is_empty());
                dto.department   = parts.get(1).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty());
            }
            "TITLE" => { dto.job_title = Some(vcard_unescape(value)); }
            "NICKNAME" => { dto.nickname = Some(vcard_unescape(value)); }
            "EMAIL" => {
                let t = extract_type(params, "internet").to_lowercase();
                dto.emails.push(ContactField { label: None, value: value.trim().to_string(), field_type: t });
            }
            "TEL" => {
                let t = extract_type(params, "voice").to_lowercase();
                dto.phones.push(ContactField { label: None, value: value.trim().to_string(), field_type: t });
            }
            "ADR" => {
                let t = extract_type(params, "home").to_lowercase();
                let parts: Vec<&str> = value.splitn(7, ';').collect();
                dto.addresses.push(AddressField {
                    label: None,
                    field_type: t,
                    street:   parts.get(2).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty()),
                    city:     parts.get(3).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty()),
                    region:   parts.get(4).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty()),
                    postcode: parts.get(5).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty()),
                    country:  parts.get(6).map(|s| vcard_unescape(s)).filter(|s| !s.is_empty()),
                });
            }
            "URL" => {
                dto.urls.push(ContactField { label: None, value: value.trim().to_string(), field_type: "web".to_string() });
            }
            "BDAY" => {
                dto.dates.push(DateField { label: None, field_type: "birthday".to_string(), value: value.trim().to_string() });
            }
            "NOTE" => { dto.notes = Some(vcard_unescape(value)); }
            _ => {}
        }
    }

    // Need at least a name or email
    if dto.display_name.is_none() && dto.given_name.is_none() && dto.family_name.is_none() && dto.emails.is_empty() {
        return None;
    }
    Some(dto)
}

fn extract_type(params: &str, default: &str) -> String {
    for part in params.split(';') {
        if let Some(val) = part.strip_prefix("TYPE=") {
            return val.split(',').next().unwrap_or(default).to_lowercase();
        }
    }
    default.to_string()
}

fn vcard_unescape(s: &str) -> String {
    s.replace("\\n", "\n")
     .replace("\\N", "\n")
     .replace("\\,", ",")
     .replace("\\;", ";")
     .replace("\\\\", "\\")
}
