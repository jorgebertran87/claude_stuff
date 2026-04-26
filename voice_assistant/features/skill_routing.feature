Feature: Skill routing by order content
  As a user
  I want my order to be handled by the skill that matches its topic
  So that bus, music, and weather questions get domain-specific responses

  Scenario: Order about the bus selects the bus skill
    Given an order containing "autobús"
    When the system detects the intent
    Then the selected skill is "bus"

  Scenario: Order mentioning a bus stop selects the bus skill
    Given an order containing "parada"
    When the system detects the intent
    Then the selected skill is "bus"

  Scenario: Natural language bus time question selects the bus skill
    Given an order containing "a qué hora sale el bus?"
    When the system detects the intent
    Then the selected skill is "bus"

  Scenario: Order about music selects the music skill
    Given an order containing "pon música"
    When the system detects the intent
    Then the selected skill is "music"

  Scenario: Order mentioning Spotify selects the music skill
    Given an order containing "reproduce en spotify"
    When the system detects the intent
    Then the selected skill is "music"

  Scenario: Order about the weather selects the weather skill
    Given an order containing "tiempo"
    When the system detects the intent
    Then the selected skill is "weather"

  Scenario: Order about rain selects the weather skill
    Given an order containing "mañana lloverá"
    When the system detects the intent
    Then the selected skill is "weather"

  Scenario: Unrecognised order falls back to the search skill
    Given an order containing "quién fue Cervantes"
    When the system detects the intent
    Then the selected skill is "search"
