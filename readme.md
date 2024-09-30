
# Rust AWS Lambda DynamoDB REST Backend with Firebase JWT Auth and Paginated Queries

This project provides examples of building a REST backend using Rust with aws DynamoDB 
and Firebase Auth Token authorization using Cargo Lambda for dev and release.
            
#### Run Locally:

```cargo lambda watch --invoke-port=9003```


#### Build:

```cargo lambda build```

To avoid missing OpenSSL headers issue in build process, OpenSSL with Vendored feature has been added
to cargo.toml.

#### Deploy as a custom Lambda function:

 ```cargo lambda deploy```

### Pagination

This project shows examples of pagination queries with DynamoDB and REST. AWS only returns
a Base64 key representing the 
last queried key in the AWS CLI and not in the Rust SDK. In the Rust SDK, they provide examples
of returning the last queried key in the form of a HashMap<String, AttributeValue>, but
not in the form of a Base64 key.

This project provides an example of converting the returned HashMap into a struct
that is converted to a Base64 String that can then be passed via a REST header
to the consumer. Upon querying the next set of keys, the token is provided back
to the REST call via a query parameter and the Base64 is converted back into
a readable AWS DynamoDB object for setting the query start point.

A query of the UserTable will return an ```app-token``` header value. Place this in the query as follows: 
```shell
curl -H "Content-Type: application/json" \
-X GET "http://localhost:{{port}}/dynamo_query_accountusers_handler?page_size=2&token=RETURNED_TOKEN_FROM_APP-TOKEN_IN_HEADER"
```

### Firebase Auth

To obtain a Firebase Token from your Firebase project for 
testing with the test endpoint:
```shell
curl -X POST --location "https://identitytoolkit.googleapis.com/v1/accounts:signInWithPassword?key=[YOUR_FIREBASE_KEY]"
-H "Content-Type: application/json"
-d '{
"email": "email@of.user",
"password": "password_of_user,
"returnSecureToken": true
}'
```

Then provide the token in a REST call header:

```shell
curl -v --header "Content-Type: application/json"
--header "Authorization: Bearer PLACE_FIREBASE_TOKEN_HERE"
--request GET
http://localhost:9003/get_fb_token_claims
```

You will then receive the token's claims as a response.


You can then add the authorization middleware to any route by adding
the authorize_firebase function to a layer on the route:
   ```rust
        .route(
            "/get_fb_token_claims",
            get(get_fb_token_claims)
                .layer(middleware::from_fn(auth::authorize_firebase)),
        )
   ```

Make sure to set the following Environment Variables or set them directly in the code:
``` 
JWK_URL="https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com"
JWK_AUDIENCE="FIREBASE_PROJECT_ID"
JWK_ISSUER="https://securetoken.google.com/FIREBASE_PROJECT_ID"
```
The JWT keys will be refreshed automatically, which is useful if this is run as a stand-alone instance.
If you run as purely a Lambda function, the firebase keys could be stored in an in-memory db.



### DynamoDB UserTable

To create the user table, load the ```AccountUser.json``` file into DynamoDB.
Download the AWS NoSQL Workbench for free and load the table with sample data
and publish to AWS directly from NoSQL Workbench.

#### IAM Roles

Make sure to grant access to DynamoDB to the Lambda function you deploy.
You can set access by adding Dynamo permissions to the IAM role
associated with the function:
https://aws.amazon.com/blogs/security/how-to-create-an-aws-iam-policy-to-grant-aws-lambda-access-to-an-amazon-dynamodb-table/




### More ...

Cargo Lambda  https://www.cargo-lambda.info/
   
Aws Lambda Dynamo examples based on (and you can find more examples at):

   https://github.com/awslabs/aws-lambda-rust-runtime/blob/main/examples/http-axum/src/main.rs
   https://github.com/awsdocs/aws-doc-sdk-examples/tree/main/rustv1/examples/dynamodb#code-examples

Rust Modyne (opinionated Rust Dynamo orm)

https://github.com/neoeinstein/modyne

Thanks to:
https://github.com/maylukas/rust_jwk_example for base firebase JWK implementation. Some modifications have been made here.
