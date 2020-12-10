use std::error::Error;

use sheets::Sheets;
use yup_oauth2::{read_application_secret, InstalledFlowAuthenticator, InstalledFlowReturnMethod};

pub struct Sheet {
    pub sheet_id: String,
    pub client: Sheets,
}

impl Sheet {
    pub async fn new(sheet_id: String) -> Result<Sheet, Box<dyn Error>> {
        // Get the GSuite credentials file.
        let secret = read_application_secret("./cred/credentials.json").await?;

        let auth =
            InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
                .persist_tokens_to_disk("./cred/tokencache.json")
                .build()
                .await?;

        // Add the scopes to the secret and get the token.
        let token = auth
            .token(&["https://www.googleapis.com/auth/spreadsheets"])
            .await?;

        if token.as_str().is_empty() {
            panic!("empty token is not valid");
        }

        let client = Sheets::new(token);
        Ok(Sheet { sheet_id, client })
    }

    pub async fn read_range(
        &self,
        a1_notation: &str,
    ) -> Result<Option<Vec<Vec<String>>>, Box<dyn Error>> {
        let result = self
            .client
            .get_values(&self.sheet_id, a1_notation.to_string())
            .await?;
        Ok(result.values)
    }

    pub async fn write_range(&self, a1_notation: &str, value: &str) -> Result<(), Box<dyn Error>> {
        self.client
            .update_values(&self.sheet_id, a1_notation, value.to_string())
            .await?;
        Ok(())
    }
}
