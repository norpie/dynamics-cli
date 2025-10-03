use crate::config_legacy::{AuthConfig, Config};
use crate::ui::prompts::text_input;
use anyhow::Result;
use log::{debug, info};
use reqwest::Client;
use serde_json::Value;

#[derive(Debug)]
pub struct DynamicsClient {
    client: Client,
    auth_config: AuthConfig,
    access_token: Option<String>,
}

impl DynamicsClient {
    pub fn new(auth_config: AuthConfig) -> Self {
        Self {
            client: Client::new(),
            auth_config,
            access_token: None,
        }
    }

    /// Get or refresh the access token
    async fn get_access_token(&mut self) -> Result<&str> {
        if self.access_token.is_none() {
            self.authenticate().await?;
        }

        Ok(self.access_token.as_ref().unwrap())
    }

    /// Authenticate with Azure AD to get access token
    async fn authenticate(&mut self) -> Result<()> {
        info!("Authenticating with Dynamics 365...");
        debug!(
            "Host: {}, Client ID: {}",
            self.auth_config.host, self.auth_config.client_id
        );

        let token_url = "https://login.windows.net/common/oauth2/token";

        let response = self
            .client
            .post(token_url)
            .form(&[
                ("grant_type", "password"),
                ("client_id", &self.auth_config.client_id),
                ("client_secret", &self.auth_config.client_secret),
                ("username", &self.auth_config.username),
                ("password", &self.auth_config.password),
                ("resource", &self.auth_config.host),
            ])
            .send()
            .await?;

        debug!("Token request status: {}", response.status());

        if response.status().is_success() {
            let token_data: Value = response.json().await?;
            if let Some(access_token) = token_data.get("access_token").and_then(|t| t.as_str()) {
                self.access_token = Some(access_token.to_string());
                debug!("Access token obtained successfully");
                return Ok(());
            }
            anyhow::bail!("Authentication failed: No access token in response");
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Authentication failed: {}", error_text)
        }
    }

    /// Extract entity name from FetchXML
    async fn extract_entity_name(&self, fetchxml: &str) -> Result<String> {
        let doc = roxmltree::Document::parse(fetchxml)
            .map_err(|e| anyhow::anyhow!("Failed to parse FetchXML: {}", e))?;

        // Find the entity element
        let entity_node = doc
            .descendants()
            .find(|n| n.has_tag_name("entity"))
            .ok_or_else(|| anyhow::anyhow!("No entity element found in FetchXML"))?;

        // Get the entity name attribute
        let entity_name = entity_node
            .attribute("name")
            .ok_or_else(|| anyhow::anyhow!("Entity element missing 'name' attribute"))?;

        // Convert to plural form for Web API endpoint
        let plural_name = self.pluralize_entity_name(entity_name).await?;

        debug!("Extracted entity name: {} -> {}", entity_name, plural_name);
        Ok(plural_name)
    }

    /// Convert entity name to plural form for Dynamics Web API
    async fn pluralize_entity_name(&self, entity_name: &str) -> Result<String> {
        // First check config for custom mappings
        let config = Config::load()?;
        if let Some(plural) = config.get_entity_mapping(entity_name) {
            debug!("Found custom mapping: {} -> {}", entity_name, plural);
            return Ok(plural.clone());
        }

        // Then check built-in mappings
        let built_in_plural = match entity_name {
            // Common Dynamics entities with irregular plurals
            "account" => "accounts",
            "contact" => "contacts",
            "lead" => "leads",
            "opportunity" => "opportunities",
            "campaign" => "campaigns",
            "incident" => "incidents",
            "quote" => "quotes",
            "salesorder" => "salesorders",
            "invoice" => "invoices",
            "product" => "products",
            "appointment" => "appointments",
            "task" => "tasks",
            "phonecall" => "phonecalls",
            "email" => "emails",
            "letter" => "letters",
            "fax" => "faxes",
            "activitypointer" => "activitypointers",
            "annotation" => "annotations",
            "systemuser" => "systemusers",
            "team" => "teams",
            "businessunit" => "businessunits",
            "role" => "roles",
            _ => "",
        };

        if !built_in_plural.is_empty() {
            debug!(
                "Found built-in mapping: {} -> {}",
                entity_name, built_in_plural
            );
            return Ok(built_in_plural.to_string());
        }

        // Unknown entity - prompt user interactively
        self.prompt_for_entity_mapping(entity_name).await
    }

    /// Get entity plural name without prompting (silent mode for TUI)
    async fn pluralize_entity_name_silent(&self, entity_name: &str) -> Result<String> {
        // First check config for custom mappings
        let config = Config::load()?;
        if let Some(plural) = config.get_entity_mapping(entity_name) {
            debug!("Found custom mapping: {} -> {}", entity_name, plural);
            return Ok(plural.clone());
        }

        // Then check built-in mappings
        let built_in_plural = match entity_name {
            // Common Dynamics entities with irregular plurals
            "account" => "accounts",
            "contact" => "contacts",
            "lead" => "leads",
            "opportunity" => "opportunities",
            "campaign" => "campaigns",
            "incident" => "incidents",
            "quote" => "quotes",
            "salesorder" => "salesorders",
            "invoice" => "invoices",
            "product" => "products",
            "appointment" => "appointments",
            "task" => "tasks",
            "phonecall" => "phonecalls",
            "email" => "emails",
            "letter" => "letters",
            "fax" => "faxes",
            "activitypointer" => "activitypointers",
            "annotation" => "annotations",
            "systemuser" => "systemusers",
            "team" => "teams",
            "businessunit" => "businessunits",
            "role" => "roles",
            _ => "",
        };

        if !built_in_plural.is_empty() {
            debug!(
                "Found built-in mapping: {} -> {}",
                entity_name, built_in_plural
            );
            return Ok(built_in_plural.to_string());
        }

        // Unknown entity - use default pluralization (no prompting)
        let default_plural = format!("{}s", entity_name);
        debug!(
            "Using default pluralization for unknown entity: {} -> {}",
            entity_name, default_plural
        );
        Ok(default_plural)
    }

    /// Prompt user for entity mapping when entity is not known
    async fn prompt_for_entity_mapping(&self, entity_name: &str) -> Result<String> {
        println!("\nUnknown entity: '{}'", entity_name);
        println!("Dynamics 365 Web API requires the plural form of entity names.");

        // Suggest default pluralization
        let suggested_plural = format!("{}s", entity_name);

        println!("Suggested plural form: '{}'", suggested_plural);

        let prompt = format!(
            "Enter plural form for '{}' (or press Enter for '{}')",
            entity_name, suggested_plural
        );
        let user_input = text_input(&prompt, Some(&suggested_plural))?;

        let plural_name = if user_input.trim().is_empty() {
            suggested_plural
        } else {
            user_input.trim().to_string()
        };

        // Save the mapping for future use
        let mut config = Config::load()?;
        config.add_entity_mapping(entity_name.to_string(), plural_name.clone())?;

        println!("Saved mapping: {} -> {}", entity_name, plural_name);
        println!("You can manage entity mappings with: dynamics-cli entity");

        Ok(plural_name)
    }

    /// Execute a FetchXML query against Dynamics 365
    pub async fn execute_fetchxml(&mut self, fetchxml: &str) -> Result<Value> {
        let token = self.get_access_token().await?.to_string();

        // Extract entity name from FetchXML
        let entity_name = self.extract_entity_name(fetchxml).await?;

        // Construct the Web API URL for FetchXML queries
        let mut base_url = self.auth_config.host.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }
        let query_url = format!("{}api/data/v9.2/{}", base_url, entity_name);

        info!("Executing FetchXML query against: {}", query_url);
        debug!("FetchXML: {}", fetchxml);

        // URL encode the FetchXML
        let encoded_fetchxml = urlencoding::encode(fetchxml);
        let full_url = format!("{}?fetchXml={}", query_url, encoded_fetchxml);

        let response = self
            .client
            .get(&full_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .send()
            .await?;

        debug!("Query response status: {}", response.status());

        if response.status().is_success() {
            let result: Value = response.json().await?;
            debug!("Query executed successfully");
            Ok(result)
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Query execution failed: {}", error_text)
        }
    }

    /// Execute a FetchXML query and return formatted results
    pub async fn query(&mut self, fetchxml: &str, format: &str, pretty: bool) -> Result<String> {
        let result = self.execute_fetchxml(fetchxml).await?;

        match format {
            "json" => {
                if pretty {
                    Ok(serde_json::to_string_pretty(&result)?)
                } else {
                    Ok(serde_json::to_string(&result)?)
                }
            }
            "xml" => {
                // For XML format, return the original FetchXML along with results
                if pretty {
                    Ok(format!(
                        "<!-- FetchXML Query -->\n{}\n\n<!-- Results -->\n{}",
                        fetchxml,
                        serde_json::to_string_pretty(&result)?
                    ))
                } else {
                    Ok(format!("{}\n{}", fetchxml, serde_json::to_string(&result)?))
                }
            }
            "table" => {
                // Format as a simple table
                self.format_as_table(&result)
            }
            _ => anyhow::bail!("Unsupported format: {}", format),
        }
    }

    /// Format query results as a simple table
    fn format_as_table(&self, result: &Value) -> Result<String> {
        let mut output = String::new();

        if let Some(values) = result.get("value").and_then(|v| v.as_array()) {
            if values.is_empty() {
                return Ok("No records found.".to_string());
            }

            // Get all unique column names from the first few records
            let mut columns = std::collections::HashSet::new();
            for record in values.iter().take(10) {
                if let Some(obj) = record.as_object() {
                    for key in obj.keys() {
                        if !key.starts_with('@') && !key.starts_with('_') {
                            columns.insert(key.clone());
                        }
                    }
                }
            }

            let mut column_vec: Vec<String> = columns.into_iter().collect();
            column_vec.sort();

            // Header
            output.push_str(&format!("{}\n", column_vec.join(" | ")));
            output.push_str(&format!("{}\n", "-".repeat(column_vec.len() * 15)));

            // Data rows
            for record in values {
                if let Some(obj) = record.as_object() {
                    let row: Vec<String> = column_vec
                        .iter()
                        .map(|col| {
                            obj.get(col)
                                .map(|v| match v {
                                    Value::String(s) => s.clone(),
                                    Value::Number(n) => n.to_string(),
                                    Value::Bool(b) => b.to_string(),
                                    Value::Null => "null".to_string(),
                                    _ => "...".to_string(),
                                })
                                .unwrap_or_else(|| "".to_string())
                        })
                        .collect();
                    output.push_str(&format!("{}\n", row.join(" | ")));
                }
            }

            output.push_str(&format!("\nTotal records: {}\n", values.len()));
        } else {
            output.push_str("Invalid response format\n");
        }

        Ok(output)
    }

    /// Generic GET request to Dynamics API
    pub async fn get(&mut self, endpoint: &str) -> Result<String> {
        let token = self.get_access_token().await?.to_string();

        // Construct the full URL
        let mut base_url = self.auth_config.host.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }

        let full_url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("{}api/data/v9.2/{}", base_url, endpoint)
        };

        debug!("GET request to: {}", full_url);

        let response = self
            .client
            .get(&full_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .send()
            .await?;

        let status = response.status();
        debug!("Response status: {}", status);

        let response_text = response.text().await?;
        debug!("Response body length: {} chars", response_text.len());

        if status.is_success() {
            Ok(response_text)
        } else {
            Err(anyhow::anyhow!(
                "HTTP request failed: {} - {}",
                status, response_text
            ))
        }
    }

    /// Fetch entity metadata from Dynamics 365
    pub async fn fetch_metadata(&mut self) -> Result<String> {
        let token = self.get_access_token().await?.to_string();

        // Construct the metadata URL
        let mut base_url = self.auth_config.host.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }
        let metadata_url = format!("{}api/data/v9.2/$metadata", base_url);

        info!("Fetching metadata from: {}", metadata_url);

        let response = self
            .client
            .get(&metadata_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/xml")
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .send()
            .await?;

        debug!("Metadata response status: {}", response.status());

        if response.status().is_success() {
            let metadata_xml = response.text().await?;
            debug!("Metadata fetched successfully");
            Ok(metadata_xml)
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Metadata fetch failed: {}", error_text)
        }
    }

    /// Fetch views from Dynamics 365 savedqueries entity
    pub async fn fetch_views(
        &mut self,
        entity_name: Option<&str>,
    ) -> Result<Vec<crate::dynamics::metadata::ViewInfo>> {
        let token = self.get_access_token().await?.to_string();

        // Construct the savedqueries URL
        let mut base_url = self.auth_config.host.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }

        // Build the OData query for savedqueries
        let mut query_url = format!("{}api/data/v9.2/savedqueries", base_url);

        // Add filter for specific entity if provided
        if let Some(entity) = entity_name {
            query_url.push_str(&format!("?$filter=returnedtypecode eq '{}'", entity));
            query_url.push_str("&$select=name,returnedtypecode,querytype,iscustom,fetchxml");
        } else {
            query_url.push_str("?$select=name,returnedtypecode,querytype,iscustom,fetchxml");
        }

        // Order by entity name and then by view name
        query_url.push_str("&$orderby=returnedtypecode,name");

        info!("Fetching views from: {}", query_url);

        let response = self
            .client
            .get(&query_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .send()
            .await?;

        debug!("Views response status: {}", response.status());

        if response.status().is_success() {
            let views_data: Value = response.json().await?;
            debug!("Views fetched successfully");

            let mut views = Vec::new();

            if let Some(value_array) = views_data.get("value").and_then(|v| v.as_array()) {
                for view_data in value_array {
                    if let Some(view_info) = self.parse_view_from_json(view_data)? {
                        views.push(view_info);
                    }
                }
            }

            debug!("Parsed {} views", views.len());
            Ok(views)
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Views fetch failed: {}", error_text)
        }
    }

    /// Parse a single view from JSON response
    fn parse_view_from_json(
        &self,
        view_data: &Value,
    ) -> Result<Option<crate::dynamics::metadata::ViewInfo>> {
        let name = view_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown View")
            .to_string();

        let entity_name = view_data
            .get("returnedtypecode")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let query_type = view_data
            .get("querytype")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let view_type = match query_type {
            0 => "Public".to_string(),
            1 => "Advanced Find".to_string(),
            2 => "Associated".to_string(),
            4 => "Quick Find".to_string(),
            _ => format!("Type {}", query_type),
        };

        let is_custom = view_data
            .get("iscustom")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let fetch_xml = view_data
            .get("fetchxml")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Skip views without FetchXML (incomplete data)
        if fetch_xml.is_empty() {
            return Ok(None);
        }

        // Parse columns from FetchXML
        let columns = crate::dynamics::metadata::parse_view_columns(&fetch_xml)
            .unwrap_or_else(|_| Vec::new());

        Ok(Some(crate::dynamics::metadata::ViewInfo {
            name,
            entity_name,
            view_type,
            is_custom,
            columns,
            fetch_xml,
        }))
    }

    /// Fetch forms from Dynamics 365 systemforms entity
    pub async fn fetch_forms(
        &mut self,
        entity_name: Option<&str>,
    ) -> Result<Vec<crate::dynamics::metadata::FormInfo>> {
        let token = self.get_access_token().await?.to_string();

        // Construct the systemforms URL
        let mut base_url = self.auth_config.host.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }

        // Build the OData query for systemforms
        let mut query_url = format!("{}api/data/v9.2/systemforms", base_url);

        // Build proper OData query with $select and $filter
        if let Some(entity) = entity_name {
            // Entity-specific query: filter by entity and form criteria
            query_url.push_str(&format!(
                "?$filter=objecttypecode eq '{}' and formactivationstate eq 1 and (type eq 2 or type eq 7 or type eq 8)",
                entity
            ));
            query_url.push_str(
                "&$select=name,objecttypecode,type,iscustomizable,formactivationstate,formxml",
            );
        } else {
            // General query: filter by form criteria only
            query_url.push_str(
                "?$filter=formactivationstate eq 1 and (type eq 2 or type eq 7 or type eq 8)",
            );
            query_url.push_str(
                "&$select=name,objecttypecode,type,iscustomizable,formactivationstate,formxml",
            );
        }

        // Order by form type and then by name
        query_url.push_str("&$orderby=type,name");

        info!("Fetching forms from: {}", query_url);

        let response = self
            .client
            .get(&query_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .send()
            .await?;

        debug!("Forms response status: {}", response.status());

        if response.status().is_success() {
            let response_text = response.text().await?;
            let forms_data: Value = serde_json::from_str(&response_text)?;

            let mut forms = Vec::new();

            if let Some(value_array) = forms_data.get("value").and_then(|v| v.as_array()) {
                for form_data in value_array {
                    if let Some(form_info) = self.parse_form_from_json(form_data)? {
                        forms.push(form_info);
                    }
                }
            }

            debug!("Parsed {} forms", forms.len());
            Ok(forms)
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Forms fetch failed: {}", error_text)
        }
    }

    /// Parse a single form from JSON response
    fn parse_form_from_json(
        &self,
        form_data: &Value,
    ) -> Result<Option<crate::dynamics::metadata::FormInfo>> {
        let name = form_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Form")
            .to_string();

        let entity_name = form_data
            .get("objecttypecode")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let form_type_code = form_data.get("type").and_then(|v| v.as_i64()).unwrap_or(0);

        let form_type = match form_type_code {
            2 => "Main".to_string(),
            7 => "QuickCreate".to_string(),
            8 => "QuickView".to_string(),
            11 => "Card".to_string(),
            _ => format!("Type{}", form_type_code),
        };

        let is_custom = form_data
            .get("iscustomizable")
            .and_then(|v| v.get("Value"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let state = form_data
            .get("formactivationstate")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let form_xml = form_data
            .get("formxml")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Skip forms without FormXML (incomplete data)
        if form_xml.is_empty() {
            return Ok(None);
        }

        // Parse form structure from FormXML
        let form_info = crate::dynamics::metadata::FormInfo {
            name: name.clone(),
            entity_name: entity_name.clone(),
            form_type,
            is_custom,
            state,
            form_xml: form_xml.clone(),
            form_structure: None, // Will be populated when needed
        };

        // Parse the structure if possible
        let form_with_structure = match crate::dynamics::metadata::parse_form_structure(&form_info)
        {
            Ok(structure) => crate::dynamics::metadata::FormInfo {
                form_structure: Some(structure),
                ..form_info
            },
            Err(_) => form_info, // Keep the form even if structure parsing fails
        };

        Ok(Some(form_with_structure))
    }

    /// Fetch a specific record by ID from Dynamics 365
    pub async fn fetch_record_by_id(&mut self, entity_name: &str, record_id: &str) -> Result<Value> {
        let token = self.get_access_token().await?.to_string();

        // Construct the base URL
        let mut base_url = self.auth_config.host.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }

        // Get the plural form of the entity name
        let plural_entity_name = self.pluralize_entity_name(entity_name).await?;

        // Construct the record URL
        let record_url = format!("{}api/data/v9.2/{}({})", base_url, plural_entity_name, record_id);

        info!("Fetching record from: {}", record_url);

        let response = self
            .client
            .get(&record_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .send()
            .await?;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!(
                "Failed to fetch record {}: HTTP {} - {}",
                record_id,
                status,
                error_text
            ))
        }
    }

    /// Fetch a specific record by ID from Dynamics 365 (silent mode for TUI)
    /// This version doesn't prompt for unknown entity names
    pub async fn fetch_record_by_id_silent(&mut self, entity_name: &str, record_id: &str) -> Result<Value> {
        let token = self.get_access_token().await?.to_string();

        // Construct the base URL
        let mut base_url = self.auth_config.host.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }

        // Get the plural form of the entity name (silent mode - no prompts)
        let plural_entity_name = self.pluralize_entity_name_silent(entity_name).await?;

        // Construct the record URL
        let record_url = format!("{}api/data/v9.2/{}({})", base_url, plural_entity_name, record_id);

        info!("Fetching record from: {}", record_url);

        let response = self
            .client
            .get(&record_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .send()
            .await?;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!(
                "Failed to fetch record {}: HTTP {} - {}",
                record_id,
                status,
                error_text
            ))
        }
    }

    /// Fetch a record by ID with formatted values for example data display
    /// Includes OData annotations for lookup field display names
    pub async fn fetch_example_record_by_id(&mut self, entity_name: &str, record_id: &str) -> Result<Value> {
        let token = self.get_access_token().await?.to_string();

        // Construct the base URL
        let mut base_url = self.auth_config.host.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }

        // Get the plural form of the entity name (silent mode - no prompts)
        let plural_entity_name = self.pluralize_entity_name_silent(entity_name).await?;

        // Construct the record URL with formatted value preference
        // The Prefer header requests formatted values for lookup fields
        let record_url = format!("{}api/data/v9.2/{}({})", base_url, plural_entity_name, record_id);

        info!("Fetching example record from: {}", record_url);

        let response = self
            .client
            .get(&record_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .header("Prefer", "odata.include-annotations=\"*\"")
            .send()
            .await?;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!(
                "Failed to fetch example record {}: HTTP {} - {}",
                record_id,
                status,
                error_text
            ))
        }
    }
}
