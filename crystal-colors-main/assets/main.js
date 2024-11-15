var ws;
var lastColor;

// async function to get called when the page is loaded
async function main() {
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
    }

    let allMessages = [];
    ws.onmessage = function (e) {
        let data = JSON.parse(e.data);
        if (Array.isArray(data)) {
            createSingleMessage(e.data);
        } else if (typeof data === 'object' && data !== null) {
            allMessages.push(data);
            createSingleMessage(JSON.stringify(allMessages));
        }
        if (!isNaN(e.data)) {
            output.style.backgroundColor = "hsl(" + e.data + ", 100%, 50%)";
            for (let i = 0; i < singleMessageBox.length; i++) {
                singleMessageBox[i].style.backgroundColor = "hsl(" + e.data + ", 100%, 50%)";
            }
            slider.value = e.data;
            lastColor = e.data;
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

function checkInputs() {
    let inputfield = document.getElementById("inputfield");
    let userinput = document.getElementById("userInput");
    let message = {
        type: "Message",
        user: userinput.value,
        message: inputfield.value
    };
    inputfield.value = "";
    userinput.value = "";
    ws.send(JSON.stringify(message));
}


function createSingleMessage(messagesAsString) {
    let outputMessage = document.getElementById("chatContainer");

    if (messagesAsString) {
        let messageArray = JSON.parse(messagesAsString);

        outputMessage.innerHTML = "";

        let firstUser = messageArray[0].user;
        for (let i = 0; i < messageArray.length; i++) {
            outputMessage.innerHTML += `
        <div class="singleMessage ${messageArray[i].user === firstUser ? `left` : `right`}">
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

function signIn() {
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
    ws.send(JSON.stringify(new_user));

    fullscreen_signIn.classList.add("d-none");
    mainWindow.classList.remove("d-none");
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
    ws.send(JSON.stringify(loginData));
}



// call the main function
document.addEventListener("DOMContentLoaded", main);
