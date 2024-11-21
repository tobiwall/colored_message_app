
var ws;
var lastColor;
var currentUser;
// async function to get called when the page is loaded
async function main() {
    currentUser = JSON.parse(localStorage.getItem("currentUser"));
    // get room id from the url (localhost:8080/room/1234 or localhost:8080/room/1234/ or localhost:8080/room/1234/index.html)
    // get the part of the url after the first "room" part
    const roomId = (window.location.pathname.split("/"))[2];
    var slider = document.getElementById("hueInput");
    var singleMessageBox = document.getElementsByClassName("singleMessage");
    var output = document.getElementById("hueDiv");
    // open a ws connection to "/echo" and send a message every second
    var protocol = location.protocol === "https:" ? "wss:" : "ws:";
    ws = new WebSocket(protocol + "//" + location.host + "/echo/" + roomId);

    let allMessages = [];

    ws.onmessage = async function (e) {
        let data;
        try {
            data = JSON.parse(e.data);
        } catch {
            console.error(e.data);
            return;
        }

        switch (data.type) {
            case "LoginResponse":
                await handleLoginResponse(data);
                login_message_arrived = true;
                break;

            case "NewUserResponse":
                handleNewUserResponse(data);
                signup_message_arrived = true;
                break;

            case "MessageResponse":
                handleMessageResponse(data, allMessages);
                break;

            case "Color":
                setBackgroundColor(data.value, output, singleMessageBox, slider);
                break;

            default:
                console.warn("Unknown message type:", data.type);
        }
    };

    // Update the current slider value (each time you drag the slider handle)
    slider.oninput = function () {
        // set background color to the current value
        output.style.backgroundColor = "hsl(" + this.value + ", 100%, 50%)";
        for (let i = 0; i < singleMessageBox.length; i++) {
            if (!this.value) {
                this.value = 180;
            }
            singleMessageBox[i].style.backgroundColor = "hsl(" + this.value + ", 100%, 50%)";
        }
        lastColor = this.value;

        const color = {
            type: "Color",
            value: this.value
        }
        ws.send(JSON.stringify(color));
    }

}

function waitForSocketOpen(socket) {
    return new Promise((resolve, reject) => {
        if (socket.readyState === WebSocket.OPEN) {
            resolve();
        } else {
            socket.addEventListener('open', () => {
                resolve();
            });

            setTimeout(() => {
                if (socket.readyState !== WebSocket.OPEN) {
                    reject(new Error('WebSocket connection timed out.'));
                }
            }, 5000);
        }
    })
}

async function handleLoginResponse(login) {
    if (login.success == true) showMainScreen();
    else localStorage.setItem("login_success", false);
    console.log("This is login: " + login.login_message);
    showPopup(login);
    // alert(login.login_message);
}

async function handleNewUserResponse(signup) {
    console.log(signup.signup_message);
    if (signup.success == true) {
        showMainScreen();
    }
}

async function handleMessageResponse(message, allMessages) {
    allMessages.push(message);
    createSingleMessage(JSON.stringify(allMessages));
}

function setBackgroundColor(data, output, singleMessageBox, slider) {
    output.style.backgroundColor = "hsl(" + data + ", 100%, 50%)";
    for (let i = 0; i < singleMessageBox.length; i++) {
        singleMessageBox[i].style.backgroundColor = "hsl(" + data + ", 100%, 50%)";
    }
    if (!isNaN(data)) {
        console.log("data: " + data);

        slider.value = data;
        lastColor = data;
        localStorage.setItem("currentColor", lastColor);
    }
}

function checkInputs() {
    let inputfield = document.getElementById("inputfield");
    let message = {
        type: "Message",
        user: currentUser,
        message: inputfield.value
    };
    inputfield.value = "";
    ws.send(JSON.stringify(message));
}

function createSingleMessage(messagesAsString) {
    let outputMessage = document.getElementById("chatContainer");
    if (messagesAsString) {
        let messageArray = JSON.parse(messagesAsString);
        outputMessage.innerHTML = "";
        for (let i = 0; i < messageArray.length; i++) {
            outputMessage.innerHTML += `
        <div class="singleMessage ${messageArray[i].user === currentUser ? `left` : `right`}">
            <p class="user">${messageArray[i].user}</p>
            <p>${messageArray[i].chat_message}</p>
        </div>
    `;
        }
        if (!lastColor) {
            console.log("This is the standard color start");
            let colorLocalStorage = localStorage.getItem("currentColor");
            console.log("This is the standard color middle: " + colorLocalStorage);
            if (colorLocalStorage) {
                lastColor = colorLocalStorage;
                var output = document.getElementById("hueDiv");
                var slider = document.getElementById("hueInput");
                output.style.backgroundColor = "hsl(" + lastColor + ", 100%, 50%)";
                slider.value = lastColor;
            }
            else {
                console.log("This is the standard color");

                lastColor = 180;
            }
        }
        let singleMessageElements = document.querySelectorAll('.singleMessage');
        singleMessageElements.forEach((element) => {
            element.style.backgroundColor = "hsl(" + lastColor + ", 100%, 50%)";
        });
    }
}

async function signUp() {
    let inputName = document.getElementById("inputName");
    let inputPassword = document.getElementById("inputPassword");
    let name = inputName.value;
    let password = inputPassword.value;

    const new_user = {
        type: "NewUser",
        name: name,
        password: password
    };
    await main();
    await waitForSocketOpen(ws);
    ws.send(JSON.stringify(new_user));
    currentUser = new_user.name;
    let currentUserAsString = JSON.stringify(currentUser);
    localStorage.setItem("currentUser", currentUserAsString);
    inputName.value = "";
    inputPassword.value = "";
}

function changeToLogin() {
    let fullscreen_signIn = document.getElementById("signin");
    let fullscreen_login = document.getElementById("login");
    fullscreen_signIn.classList.add("d-none");
    fullscreen_login.classList.remove("d-none");
}

async function login() {
    let inputName = document.getElementById("inputName_login");
    let inputPassword = document.getElementById("inputPassword_login");
    let name = inputName.value;
    let password = inputPassword.value;

    const loginData = {
        type: "Login",
        name: name,
        password: password
    }

    let currentUserAsString = JSON.stringify(loginData.name);
    localStorage.setItem("loginBool", "true");
    localStorage.setItem("currentUser", currentUserAsString);
    await main();
    await waitForSocketOpen(ws);
    ws.send(JSON.stringify(loginData));
    inputName.value = "";
    inputPassword.value = "";
}

// call the main function
document.addEventListener("DOMContentLoaded", reload);

function reload() {
    let loginBool = localStorage.getItem("loginBool") === "true";
    if (loginBool) {
        main();
        showMainScreen();
    }
    console.log("This is lastcolor: " + lastColor);

}

function showMainScreen() {
    let fullscreen_login = document.getElementById("login");
    let fullscreen_signIn = document.getElementById("signin");
    let mainWindow = document.getElementById("mainWindow");
    fullscreen_login.classList.add("d-none");
    fullscreen_signIn.classList.add("d-none");
    mainWindow.classList.remove("d-none");
}

function showPopup(login) {
    let fullscreen_login = document.getElementById("login");
    let fullscreen_signIn = document.getElementById("signin");
    let mainWindow = document.getElementById("mainWindow");
    let popup = document.getElementById("popup");
    fullscreen_login.classList.add("d-none");
    fullscreen_signIn.classList.add("d-none");
    mainWindow.classList.add("d-none");
    popup.classList.remove("d-none");
    popup.innerHTML = "";
    popup.innerHTML += `
        <h3>${login.login_message}<h3>
    `;
    setTimeout(() => {
        popup.classList.add("d-none");
        if (login.success == true) {
            mainWindow.classList.remove("d-none");
        } else {
            fullscreen_login.classList.remove("d-none");
        }
    }, 2000);
}

function checkOut() {
    localStorage.removeItem("loginBool");
    localStorage.removeItem("currentUser");
    let fullscreen_login = document.getElementById("login");
    let fullscreen_signIn = document.getElementById("signin");
    let mainWindow = document.getElementById("mainWindow");
    fullscreen_login.classList.add("d-none");
    fullscreen_signIn.classList.remove("d-none");
    mainWindow.classList.add("d-none");
}
