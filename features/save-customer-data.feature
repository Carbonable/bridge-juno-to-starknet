Feature: Save customer data when token transfer happen in frontend

    Scenario: Customer just transfered his token frontend side
        Given a request
            | keplr-wallet-id | project_id | tokens          |
            | k3plr-id        | proj3ct1d  | [344, 345, 346] |
        When I execute the request
        Then data should have been persisted to database
