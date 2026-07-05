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

  Scenario: Order containing "busques" is not mistaken for bus
    Given an order containing "busques información urbanística"
    When the system detects the intent
    Then the selected skill is "search"

  Scenario: Order containing the standalone word "bus" selects bus
    Given an order containing "cuándo pasa el bus"
    When the system detects the intent
    Then the selected skill is "bus"

  Scenario: Order containing "autobús" still selects bus
    Given an order containing "el autobús 14"
    When the system detects the intent
    Then the selected skill is "bus"

  Scenario: Order about a catastral reference selects catastro
    Given an order containing "referencia catastral 5989208UF6558N0003PX"
    When the system detects the intent
    Then the selected skill is "catastro"

  Scenario: Order mentioning catastro selects catastro
    Given an order containing "consulta el catastro de esta finca"
    When the system detects the intent
    Then the selected skill is "catastro"

  Scenario: Order asking to investigate a catastral reference selects catastro
    Given an order containing "investiga sobre esta referencia catastral"
    When the system detects the intent
    Then the selected skill is "catastro"
