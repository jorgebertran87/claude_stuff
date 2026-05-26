Feature: HTML tag stripping
  As the /check command formatter
  I want to strip HTML tags and collapse whitespace from outerHTML
  So that Telegram notifications show clean readable text

  Scenario: Plain text passes through unchanged
    When I strip tags from "hello world"
    Then the stripped result is "hello world"

  Scenario: A single HTML tag is removed
    When I strip tags from "<p>Hello</p>"
    Then the stripped result is "Hello"

  Scenario: Nested tags are all removed
    When I strip tags from "<div><span>text</span></div>"
    Then the stripped result is "text"

  Scenario: Mixed content keeps only the text
    When I strip tags from "<p>Hello <b>world</b></p>"
    Then the stripped result is "Hello world"

  Scenario: Multiple whitespace tokens are collapsed to a single space
    When I strip tags from "<p>  lots   of   spaces  </p>"
    Then the stripped result is "lots of spaces"

  Scenario: &amp; entity is decoded to &
    When I strip tags from "&amp;"
    Then the stripped result is "&"

  Scenario: &lt; entity is decoded to <
    When I strip tags from "&lt;"
    Then the stripped result is "<"

  Scenario: &gt; entity is decoded to >
    When I strip tags from "&gt;"
    Then the stripped result is ">"

  Scenario: &quot; entity is decoded to a double-quote
    When I strip tags from "&quot;"
    Then the stripped result is "\""

  Scenario: &#39; entity is decoded to a single-quote
    When I strip tags from "&#39;"
    Then the stripped result is "'"
