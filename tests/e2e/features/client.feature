Feature: NAS Sync client
  Scenario: Connect a NAS found on the network after checking its certificate
    Given the client is open without a NAS
    When I look for the NAS on the network and select it
    And I test the connection
    Then the fingerprint of the NAS is shown
    When I confirm the fingerprint
    And I sign in with a wrong password
    Then the client says the credentials are wrong
    When I sign in with the right password
    Then the synced folders of the NAS are shown

  Scenario: Create, pause and remove a sync
    Given the client is connected to the NAS
    When I create a sync of my Photos folder with the NAS
    Then the sync is listed as synced
    When I pause the sync
    Then the sync is listed as paused
    When I remove the sync
    Then no folder is synced

  Scenario: An unreachable NAS asks to check the connection
    Given the client is connected to the NAS
    When the NAS stops answering
    Then the client asks to check the network connection

  Scenario: The options change the address and disconnect the NAS
    Given the client is connected to the NAS
    When I change the address of the NAS in the options
    Then the new address is shown
    When I disconnect the NAS in the options
    Then the connection wizard is shown

  Scenario: The tray summary pauses and resumes the sync
    Given the tray summary is open with recent files
    Then the summary shows the state and the recent files
    When I pause the sync from the summary menu
    Then the summary shows the sync paused
