Feature: Google Sheets integration
  As the system
  I want to read data from a Google Spreadsheet using real OAuth credentials
  So that the assistant can answer questions about the user's personal data

  Scenario: SheetsClient is built from environment variables
    Given the Google Sheets environment variables are set
    When SheetsClient::from_env is called
    Then it returns a valid SheetsClient

  Scenario: auth_url builds a valid OAuth URL
    Given the GOOGLE_CLIENT_ID environment variable is set
    When auth_url is called
    Then the result is a URL containing the client ID
    And the result contains "scope=https://www.googleapis.com/auth/spreadsheets.readonly"

  Scenario: Fetching spreadsheet data returns tab-separated rows
    Given a valid SheetsClient built from environment variables
    When fetch_as_text is called
    Then the result is a non-empty string containing tabs and newlines

  Scenario: SheetsClient returns None when GOOGLE_SPREADSHEET_ID is missing
    Given the GOOGLE_SPREADSHEET_ID environment variable is unset
    When SheetsClient::from_env is called
    Then it returns None
