
var ws;
var lastColor;
var currentUser;
let allUsers = [];
let allMessages = [];
currentOffset = 0;
addedMessage = 0;

// async function to get called when the page is loaded
async function main() {
    currentUser = JSON.parse(localStorage.getItem("currentUser"));
    // get room id from the url (localhost:8080/room/1234 or localhost:8080/room/1234/ or localhost:8080/room/1234/index.html)
    const roomId = (window.location.pathname.split("/"))[2];
    var slider = document.getElementById("hueInput");
    var singleMessageBox = document.getElementsByClassName("singleMessage");
    var checkout = document.getElementById("checkout");
    var moreMsg = document.getElementById("moreMsg");
    var output = document.getElementById("hueDiv");
    // open a ws connection to "/echo" and send a message every second
    var protocol = location.protocol === "https:" ? "wss:" : "ws:";
    ws = new WebSocket(protocol + "//" + location.host + "/echo/" + roomId);

    ws.onmessage = async function (e) {
        let data;
        try {
            data = JSON.parse(e.data);
        } catch {
            console.error(e.data);
            return;
        }
        switch (data.type) {
            case "NewUserResponse":
                handleNewUserResponse(data);
                signup_message_arrived = true;
                break;
            case "MessageResponse":
                handleMessageResponse(data, allMessages);
                break;

            case "Color":
                setBackgroundColor(data.value, output, singleMessageBox, slider, checkout, moreMsg);
                break;

            case "AllUsers":
                allUsers.push(data);
                break;

            default:
                if (data.Success) await handleLoginResponse(data.Success, true);
                else if (data.Failure) await handleLoginResponse(data.Failure, false);
                else console.warn("Unknown message type:", data.type);
                login_message_arrived = true;
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
            checkout.style.backgroundColor = "hsl(" + this.value + ", 100%, 50%)";
            moreMsg.style.backgroundColor = "hsl(" + this.value + ", 100%, 50%)";
        }
        lastColor = this.value;

        const color = {
            type: "Color",
            value: this.value
        }
        ws.send(JSON.stringify(color));
    }
    console.log(allUsers);

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

async function handleLoginResponse(login_result, success) {
    let userLogedin = localStorage.getItem("userLogedin");
    if (success == true) {
        showMainScreen();
        localStorage.setItem("user_id", login_result);
        showPopup("You loged in successfully", true);
    }
    else {
        showPopup(login_result, false);
    }
}

async function handleNewUserResponse(signup) {
    if (signup.user_id) {
        showMainScreen();
        localStorage.setItem("user_id", signup.user_id);
        showPopup("You signed in successfully", true);
    } else {
        showPopup("User already exist!", false);
    }
}

async function handleMessageResponse(message, allMessages) {
    for (let i = 0; i < allMessages.length; i++) {
        const messageExists = allMessages.some(msg => msg.msg_id === message.msg_id);
        if (!messageExists) {
            allMessages.push(message);
        }
    }
    allMessages.sort((a, b) => b.msg_id - a.msg_id);
    console.log(allMessages);
    createSingleMessage(JSON.stringify(allMessages));
}

function setBackgroundColor(data, output, singleMessageBox, slider, checkout, moreMsg) {
    output.style.backgroundColor = "hsl(" + data + ", 100%, 50%)";
    checkout.style.backgroundColor = "hsl(" + data + ", 100%, 50%)";
    moreMsg.style.backgroundColor = "hsl(" + data + ", 100%, 50%)";
    for (let i = 0; i < singleMessageBox.length; i++) { singleMessageBox[i].style.backgroundColor = "hsl(" + data + ", 100%, 50%)"; }
    if (!isNaN(data)) {
        slider.value = data;
        lastColor = data;
        localStorage.setItem("currentColor", lastColor);
    }
}

function checkInputs() {
    let inputfield = document.getElementById("inputfield");
    let user_id = localStorage.getItem("user_id");
    let message = {
        type: "Message",
        user_id: Number(user_id),
        user: currentUser,
        message: inputfield.value
    };
    inputfield.value = "";
    ws.send(JSON.stringify(message));
}

function createSingleMessage(messagesAsString) {
    if (messagesAsString) {
        renderMessageBox(messagesAsString);
        if (!lastColor) {
            let colorLocalStorage = localStorage.getItem("currentColor");
            if (colorLocalStorage) setColorAndSlider(colorLocalStorage);
            else lastColor = 180;
        }
        let singleMessageElements = document.querySelectorAll('.singleMessage');
        singleMessageElements.forEach((element) => {
            element.style.backgroundColor = "hsl(" + lastColor + ", 100%, 50%)";
        });
    }
}

function renderMessageBox(messagesAsString) {
    let outputMessage = document.getElementById("chatContainer");
    let messageArray = JSON.parse(messagesAsString);

    outputMessage.innerHTML = "";
    for (let i = 0; i < messageArray.length; i++) {
        let user_id;
        if (messageArray[i].user) user_id = messageArray[i].user
        else if (messageArray[i].user_id) user_id = messageArray[i].user_id
        let user = getUserName(user_id);
        outputMessage.innerHTML += `
        <div class="singleMessage ${user === currentUser ? `left` : `right`}">
            <p class="user">${user}</p>
            <p>${messageArray[i].chat_message}</p>
        </div>
        `;
    }
}

function getUserName(id) {
    let foundUser = allUsers.filter(user => user.user_id === id);
    return foundUser[0].user_name;
}

function setColorAndSlider(colorLocalStorage) {
    lastColor = colorLocalStorage;
    var output = document.getElementById("hueDiv");
    var slider = document.getElementById("hueInput");
    output.style.backgroundColor = "hsl(" + lastColor + ", 100%, 50%)";
    slider.value = lastColor;
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
    localStorage.setItem("userLogedin", "true");
    await main();
    await waitForSocketOpen(ws);
    ws.send(JSON.stringify(new_user));
    currentUser = new_user.name;
    let currentUserAsString = JSON.stringify(currentUser);
    localStorage.setItem("currentUser", currentUserAsString);
    inputName.value = "";
    inputPassword.value = "";
    loadMessages();
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
    setTimeout(() => {
        localStorage.setItem("userLogedin", "true");
    }, 1000);
    localStorage.setItem("currentUser", currentUserAsString);
    await main();
    await waitForSocketOpen(ws);
    ws.send(JSON.stringify(loginData));
    inputName.value = "";
    inputPassword.value = "";
    loadMessages();
}

// call the main function
document.addEventListener("DOMContentLoaded", reload);

async function reload() {
    let userLogedin = localStorage.getItem("userLogedin") === "true";
    if (userLogedin) {
        let newMessages = await loadMessages();
        for (let i = 0; i < newMessages.length; i++) {
            allMessages.push(newMessages[i]);
        }
        main();
        showMainScreen();
    }
}

function showMainScreen() {
    let fullscreen_login = document.getElementById("login");
    let fullscreen_signIn = document.getElementById("signin");
    let mainWindow = document.getElementById("mainWindow");
    fullscreen_login.classList.add("d-none");
    fullscreen_signIn.classList.add("d-none");
    mainWindow.classList.remove("d-none");
}

function showPopup(failure, login_result) {
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
        <h3>${failure}<h3>
    `;
    setTimeout(() => {
        popup.classList.add("d-none");
        if (login_result == true) mainWindow.classList.remove("d-none");
        else {
            fullscreen_login.classList.remove("d-none");
            localStorage.removeItem("userLogedin");
            localStorage.removeItem("currentUser");
        }
    }, 2000);
}

function checkOut() {
    localStorage.removeItem("userLogedin");
    localStorage.removeItem("currentUser");
    localStorage.removeItem("user_id");
    let fullscreen_login = document.getElementById("login");
    let fullscreen_signIn = document.getElementById("signin");
    let mainWindow = document.getElementById("mainWindow");
    let absolutBotom = document.getElementById("absolutBotom");
    fullscreen_login.classList.add("d-none");
    mainWindow.classList.add("d-none");
    absolutBotom.classList.add("d-none");
    fullscreen_signIn.classList.remove("d-none");

    console.log(fullscreen_login.classList);
    console.log(mainWindow.classList);
    console.log(fullscreen_signIn.classList);
}

async function offsetPlus() {
    let newOffset = localStorage.getItem("offset") ? parseInt(localStorage.getItem("offset"), 10) : 0;
    newOffset = currentOffset += 2;
    localStorage.setItem("offset", newOffset);
    let newMessage = await loadMessages(newOffset);
    let count = 0;
    for (let i = 0; i < newMessage.length; i++) {
        const messageExists = allMessages.some(msg => msg.msg_id === newMessage[i].msg_id);
        if (!messageExists) {
            allMessages.push(newMessage[i]);
        } else {
            count += 1;
        }
    }
    if (count == 2) {
        offsetPlus();
    } else {
        allMessages.sort((a, b) => b.msg_id - a.msg_id);
        createSingleMessage(JSON.stringify(allMessages));
    }
}

async function loadMessages(offset) {
    let newMessages = [];
    if (offset) {
        currentOffset = offset;
    }
    const limit = 2;
    const response = await fetch(`/messages?limit=${limit}&offset=${currentOffset}`);
    if (response.ok) {
        let messages = await response.json();
        console.log(messages);

        for (let i = 0; i < messages.length; i++) {
            localStorage.setItem("msg_id", messages[i].msg_id);
            let newMessage = {
                chat_message: messages[i].message,
                msg_id: messages[i].msg_id,
                type: "MessageResponse",
                user: messages[i].user_id,
            }
            newMessages.push(newMessage);
        }
    } else {
        console.error("Failed to load messages", response.statusText);
    }
    return newMessages;
}