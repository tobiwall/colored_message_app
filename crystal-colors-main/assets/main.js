var ws;

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
        // generate a random number to send to the server
        // const number = Math.floor(Math.random() * 100);
        // setInterval(function () {
        //     // send a keep-alive message to the server
        //     ws.send(new Date().toUTCString());
        // }, 1000);
    }

    ws.onmessage = function (e) {
        console.log(e.data);
        // check that the message is a number
        if (isNaN(e.data)) {
            createSingleMessage(e.data);
        } else {
            // get the value from the server and set the background color
            output.style.backgroundColor = "hsl(" + e.data + ", 100%, 50%)";
            for (let i = 0; i < singleMessageBox.length; i++) {
                singleMessageBox[i].style.backgroundColor = "hsl(" + e.data + ", 100%, 50%)";
            }
            slider.value = e.data;
        }
    }

    // Update the current slider value (each time you drag the slider handle)
    slider.oninput = function () {
        // set background color to the current value
        output.style.backgroundColor = "hsl(" + this.value + ", 100%, 50%)";
        for (let i = 0; i < singleMessageBox.length; i++) {
            singleMessageBox[i].style.backgroundColor = "hsl(" + this.value + ", 100%, 50%)";
        }
        // send the value to the server
        ws.send(this.value);
    }
}

let allMessages = [];
function checkInputs() {
    let inputfield = document.getElementById("inputfield");
    let userinput = document.getElementById("userInput");
    let message = [
        user = userinput.value,
        userMessage = inputfield.value,
    ];
    allMessages.push(message);

    inputfield.value = "";
    userinput.value = "";

    let messagesAsString = JSON.stringify(allMessages);
    console.log(messagesAsString);

    ws.send(messagesAsString);
}


function createSingleMessage(messagesAsString) {
    let outputMessage = document.getElementById("chatContainer");

    if (messagesAsString) {
        let messageArray = JSON.parse(messagesAsString);

        allMessages = messageArray;

        outputMessage.innerHTML = "";
        let firstUser = messageArray[0][0];
        for (i = 0; i < messageArray.length; i++) {
            outputMessage.innerHTML += `
        <div class="singleMessage ${messageArray[i][0] === firstUser ? `left` : `right`}">
            <p class="user">${messageArray[i][0]}</p>
            <p>${messageArray[i][1]}</p>
        </div>
    `;
        }
    }
}



// call the main function
document.addEventListener("DOMContentLoaded", main);
