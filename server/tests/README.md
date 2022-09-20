# Stratum Tests

## Signal Testing

There are a number of tests to ensure that our safe signaling processing is handled correctly. Due to the nature of these tests (Throwing signals into the process in order to test our catching of them), they need to be isolated from the other tests so as to not interrupt them before concluding. 

These tests have been marked as "ignored" so that they do not run during the normal testing suite `cargo test`. Instead, in order to run these tests, the following command is recomended: `cargo test -- --ignored`

