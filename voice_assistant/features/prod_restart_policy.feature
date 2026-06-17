Feature: Production container restarts automatically on failure
  As an operator
  I want the voice assistant container to restart automatically when it crashes
  So that the service is self-healing without manual intervention

  Scenario: Container is launched with restart policy
    Given run.sh is invoked with a RUN_IMAGE
    When the container is started
    Then the docker run command includes "--restart unless-stopped"

  Scenario: Container has a healthcheck
    Given the voice assistant Docker image is built
    When the container is inspected
    Then it has a healthcheck defined

  Scenario: Container is given a name for easier management
    Given run.sh is invoked
    When the container is started
    Then the container is named "voice-assistant"
