function register () {
    let username = document.getElementById('username').value;
    if (username === "") {
        alert("Please enter a username");
        return;
    }

    fetch('/register_start/' + encodeURIComponent(username), {
        method: 'POST'
    })
    .then(response => response.json() )
    .then(credentialCreationOptions => {
        credentialCreationOptions.publicKey.challenge = Base64.toUint8Array(credentialCreationOptions.publicKey.challenge);
        credentialCreationOptions.publicKey.user.id = Base64.toUint8Array(credentialCreationOptions.publicKey.user.id);
        credentialCreationOptions.publicKey.excludeCredentials?.forEach(function (listItem) {
            listItem.id = Base64.toUint8Array(listItem.id)
        });

        return navigator.credentials.create({
            publicKey: credentialCreationOptions.publicKey
        });
    })
    .then((credential) => {
        fetch('/register_finish', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                id: credential.id,
                rawId: Base64.fromUint8Array(new Uint8Array(credential.rawId), true),
                type: credential.type,
                response: {
                    attestationObject: Base64.fromUint8Array(new Uint8Array(credential.response.attestationObject), true),
                    clientDataJSON: Base64.fromUint8Array(new Uint8Array(credential.response.clientDataJSON), true),
                },
            })
        })
        .then((response) => {
            const flash_message = document.getElementById('flash_message');
            if (response.ok){
                flash_message.innerHTML = "Successfully registered!";
            } else {
                flash_message.innerHTML = "Error whilst registering!";
            }
        });
    })
}

function login() {
    const flash_message = document.getElementById('flash_message');
    let username = document.getElementById('username');
    if (username.value === "") {
        //alert("Please enter a username");
        flash_message.innerHTML = "Please enter a username";
        return;
    }

    fetch('/login_start/' + encodeURIComponent(username.value), {
        method: 'POST'
    })
    .then(response => {
        if (response.ok){
            // location.reload();
            response.json()
                .then((credentialRequestOptions) => {
                    credentialRequestOptions.publicKey.challenge = Base64.toUint8Array(credentialRequestOptions.publicKey.challenge);
                    credentialRequestOptions.publicKey.allowCredentials?.forEach(function (listItem) {
                        listItem.id = Base64.toUint8Array(listItem.id)
                    });

                    return navigator.credentials.get({
                        publicKey: credentialRequestOptions.publicKey
                    });
                })
                .then((assertion) => {
                    fetch('/login_finish', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json'
                        },
                        body: JSON.stringify({
                            id: assertion.id,
                            rawId: Base64.fromUint8Array(new Uint8Array(assertion.rawId), true),
                            type: assertion.type,
                            response: {
                                authenticatorData: Base64.fromUint8Array(new Uint8Array(assertion.response.authenticatorData), true),
                                clientDataJSON: Base64.fromUint8Array(new Uint8Array(assertion.response.clientDataJSON), true),
                                signature: Base64.fromUint8Array(new Uint8Array(assertion.response.signature), true),
                                userHandle: Base64.fromUint8Array(new Uint8Array(assertion.response.userHandle), true)
                            },
                        }),
                    })
                        .then((response) => {
                            if (response.ok) {
                                //location.reload();
                                // flash_message.innerHTML = "Successfully logged in!";
                            } else {
                                flash_message.innerHTML = "Error whilst logging in!";
                            }
                        })
                });
        }
        else if (response.status === 404) {
            //username.value = "";
            response.text().then((bad_username) =>
                flash_message.innerHTML = "User not found: " + bad_username
            );
        }
        else {
            //username.value = "";
            flash_message.innerHTML = "Error whilst logging in!";
        }
    });
}