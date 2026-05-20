use oauth2::{
    AuthUrl,
    ClientId,
    CsrfToken,
    RedirectUrl,
    Scope
};
use oauth2::basic::BasicClient;
use tracing::error;
use url::Url;
use crate::get_config;

const AUTH_URL: &'static str = "https://anilist.co/api/v2/oauth/authorize";
pub fn get_oauth2_token(client_id: Option<u32>) -> anyhow::Result<CsrfToken> {
    let client_id = match client_id {
        Some(id) => ClientId::new(id.to_string()),
        None => {
            error!("No client id found");
            return Err(anyhow::anyhow!("Configuration error: no client id found"));
        }
    };

    let client = BasicClient::new(client_id)
        .set_auth_uri(AuthUrl::new(AUTH_URL.to_string())?);

    // Generate the full authorization URL.
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .use_implicit_flow()
        .url();

    // This is the URL you should redirect the user to, in order to trigger the authorization
    // process.
    println!("Browse to: {}", auth_url);

    Ok(csrf_token)
}

pub fn wait_for_oauth2_input(csrf_token: &CsrfToken) -> anyhow::Result<String> {
    use std::io::Write;

    print!("Paste the redirected URL after authorization: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    parse_oauth2_input(input.trim(), csrf_token)
}

fn parse_oauth2_input(input: &str, csrf_token: &CsrfToken) -> anyhow::Result<String> {
    if input.is_empty() {
        return Err(anyhow::anyhow!("No OAuth2 redirect URL provided"));
    }

    let params = match Url::parse(input) {
        Ok(url) => url
            .fragment()
            .or_else(|| url.query())
            .map(str::to_owned)
            .ok_or_else(|| anyhow::anyhow!("OAuth2 redirect URL did not contain response parameters"))?,
        Err(_) => input
            .trim_start_matches('#')
            .trim_start_matches('?')
            .to_string(),
    };

    let mut access_token = None;
    let mut state = None;
    let mut oauth_error = None;
    let mut oauth_error_description = None;

    for (key, value) in url::form_urlencoded::parse(params.as_bytes()) {
        match key.as_ref() {
            "access_token" => access_token = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => oauth_error = Some(value.into_owned()),
            "error_description" => oauth_error_description = Some(value.into_owned()),
            _ => {}
        }
    }

    if let Some(error) = oauth_error {
        let description = oauth_error_description
            .map(|description| format!(": {}", description))
            .unwrap_or_default();

        return Err(anyhow::anyhow!("OAuth2 authorization failed: {}{}", error, description));
    }

    match state {
        Some(state) if state.eq(csrf_token.secret()) => {}
        Some(_) => return Err(anyhow::anyhow!("OAuth2 CSRF state verification failed")),
        None => return Err(anyhow::anyhow!("OAuth2 response missing CSRF state")),
    }

    access_token.ok_or_else(|| anyhow::anyhow!("OAuth2 response missing access token"))
}