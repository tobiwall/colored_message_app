var ws;
var lastColor;
var currentUser;
messageQueue = [];

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
    ws.onopen = function () {
        console.log("Open ws" + ws);
        while (messageQueue.length > 0) {
            const message = messageQueue.shift();
            ws.send(JSON.stringify(message));
            console.log("Message send " + message);
            
        }

    }

    let allMessages = [];
    ws.onmessage = function (e) {
        let data;
        try {
            data = JSON.parse(e.data);
        } catch {
            console.error(e.data);
            return;
        }
        if (Array.isArray(data)) createSingleMessage(e.data);
        else if (typeof data === 'object' && data !== null && data.type !== 'Color') {
            allMessages.push(data);
            createSingleMessage(JSON.stringify(allMessages));
        }
        if (data.type == 'Color') {
            setBackgroundColor(data.value, output, singleMessageBox, slider);
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

function setBackgroundColor(data, output, singleMessageBox, slider) {
    output.style.backgroundColor = "hsl(" + data + ", 100%, 50%)";
    for (let i = 0; i < singleMessageBox.length; i++) {
        singleMessageBox[i].style.backgroundColor = "hsl(" + data + ", 100%, 50%)";
    }
    slider.value = data;
    lastColor = data;
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
            <p>${messageArray[i].message}</p>
        </div>
    `;
        }

        if (!lastColor) {
            lastColor = 180;
        }
        let singleMessageElements = document.querySelectorAll('.singleMessage');
        singleMessageElements.forEach((element) => {
            element.style.backgroundColor = "hsl(" + lastColor + ", 100%, 50%)";
        });
    }
}

function signUp() {
    main();
    let inputName = document.getElementById("inputName");
    let inputPassword = document.getElementById("inputPassword");
    let fullscreen_signIn = document.getElementById("signin");
    let mainWindow = document.getElementById("mainWindow");

    let name = inputName.value;
    let password = inputPassword.value;

    const new_user = {
        type: "NewUser",
        name: name,
        password: password
    };
    if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(new_user));
    } else {
        messageQueue.push(new_user);
    }
    currentUser = new_user.name;
    fullscreen_signIn.classList.add("d-none");
    mainWindow.classList.remove("d-none");
    currentUserAsString = JSON.stringify(currentUser);
    localStorage.setItem("currentUser", currentUserAsString);
}

function changeToLogin() {
    let fullscreen_signIn = document.getElementById("signin");
    let fullscreen_login = document.getElementById("login");
    fullscreen_signIn.classList.add("d-none");
    fullscreen_login.classList.remove("d-none");
}

function login() {
    let inputName = document.getElementById("inputName_login");
    let inputPassword = document.getElementById("inputPassword_login");
    let fullscreen_login = document.getElementById("login");
    let mainWindow = document.getElementById("mainWindow");
    let name = inputName.value;
    let password = inputPassword.value;
    fullscreen_login.classList.add("d-none");
    mainWindow.classList.remove("d-none");

    const loginData = {
        type: "Login",
        name: name,
        password: password
    }
    // ws.send(JSON.stringify(loginData));
    currentUserAsString = JSON.stringify(loginData.name);
    localStorage.setItem("loginBool", "true");
    localStorage.setItem("currentUser", currentUserAsString);
    main();
}

// call the main function
document.addEventListener("DOMContentLoaded", reload);

function reload() {
    let loginBool = localStorage.getItem("loginBool") === "true";
    if (loginBool) {
        main();
        let fullscreen_login = document.getElementById("login");
        let fullscreen_signIn = document.getElementById("signin");
        let mainWindow = document.getElementById("mainWindow");
        fullscreen_login.classList.add("d-none");
        fullscreen_signIn.classList.add("d-none");
        mainWindow.classList.remove("d-none");
    }
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
