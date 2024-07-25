const canvas = document.getElementById('radarCanvas');
const ctx = canvas.getContext('2d');
const center = { x: canvas.width / 2, y: canvas.height / 2 };
const radius = Math.min(canvas.width, canvas.height) / 2;
const numSections = 399; // Degrees in a circle

// Default scaling factor
let scalingFactor = parseInt(document.getElementById('scalingFactor').value, 10);
let pointsPerSection = 1200; // Original number of points per section
let scaledPointsPerSection = Math.ceil(pointsPerSection / scalingFactor);

let data = Array.from({ length: numSections }, () => Array(scaledPointsPerSection).fill(0));
let currentAngle = 0;
let socket;

// Update scaling factor and redraw plot
document.getElementById('scalingFactor').addEventListener('input', function() {
    scalingFactor = parseInt(this.value, 10);
    document.getElementById('scalingValue').textContent = scalingFactor;
    scaledPointsPerSection = Math.ceil(pointsPerSection / scalingFactor);
    updateRadarDataForScalingFactor(); // Update data to reflect new scaling factor
    drawRadarPlot(); // Redraw the plot with the updated scaling factor
});

function connectWebSocket() {
    const deviceNumber = document.getElementById('deviceNumber').value.trim();
    if (!deviceNumber) {
        alert("Please enter a device number.");
        return;
    }

    if (socket) {
        socket.close();
    }

    socket = new WebSocket(`ws://raulsyellowsubmarine.duckdns.org:9997/ws?device_number=${deviceNumber}`);

    socket.addEventListener('open', function() {
        console.log("WebSocket connected.");
    });

    socket.addEventListener('message', function(event) {
        const message = JSON.parse(event.data);
        const deviceData = message.DeviceMessage.PingMessage.Ping360.DeviceData;
        const angle = deviceData.angle;
        const intensityData = deviceData.data.map(value => value / 255); // Normalize

        updateRadarData(angle, intensityData);
        currentAngle = angle;
        drawRadarPlot();
    });

    socket.addEventListener('close', function() {
        console.log("WebSocket closed.");
    });

    socket.addEventListener('error', function(error) {
        console.error("WebSocket error:", error);
    });
}

function closeWebSocket() {
    if (socket) {
        socket.close();
        socket = null;
        console.log("WebSocket connection closed by user.");
    }
}

function updateRadarData(angle, newData) {
    // Reduce the data based on scaling factor
    const scaledData = [];
    for (let i = 0; i < scaledPointsPerSection; i++) {
        // Average or select every nth data point
        let sum = 0;
        let count = 0;
        for (let j = i * scalingFactor; j < (i + 1) * scalingFactor; j++) {
            if (j < newData.length) {
                sum += newData[j];
                count++;
            }
        }
        scaledData[i] = count > 0 ? sum / count : 0;
    }
    data[angle] = scaledData;
}

function updateRadarDataForScalingFactor() {
    // Recompute the data array based on the new scaling factor
    data = data.map(sectionData => {
        const scaledData = [];
        for (let i = 0; i < scaledPointsPerSection; i++) {
            // Average or select every nth data point
            let sum = 0;
            let count = 0;
            for (let j = i * scalingFactor; j < (i + 1) * scalingFactor; j++) {
                if (j < sectionData.length) {
                    sum += sectionData[j];
                    count++;
                }
            }
            scaledData[i] = count > 0 ? sum / count : 0;
        }
        return scaledData;
    });
}

function drawRadarPlot() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw circles
    ctx.strokeStyle = 'rgba(0, 255, 0, 0.5)';
    for (let i = 1; i <= 4; i++) {
        ctx.beginPath();
        ctx.arc(center.x, center.y, radius * i / 4, 0, 2 * Math.PI);
        ctx.stroke();
    }

    // Draw radar lines
    ctx.strokeStyle = 'rgba(0, 255, 0, 0.5)';
    for (let i = 0; i < 360; i += 30) {
        const x = center.x + radius * Math.cos(i * Math.PI / 180);
        const y = center.y + radius * Math.sin(i * Math.PI / 180);
        ctx.beginPath();
        ctx.moveTo(center.x, center.y);
        ctx.lineTo(x, y);
        ctx.stroke();
    }

    // Draw radar line for the current angle
    ctx.strokeStyle = 'rgba(255, 0, 0, 1)'; // Red color for the radar line
    const radarLineX = center.x + radius * Math.cos(currentAngle * Math.PI / 180);
    const radarLineY = center.y + radius * Math.sin(currentAngle * Math.PI / 180);
    ctx.beginPath();
    ctx.moveTo(center.x, center.y);
    ctx.lineTo(radarLineX, radarLineY);
    ctx.stroke();

    // Draw radar data
    for (let i = 0; i < data.length; i++) {
        for (let j = 0; j < data[i].length; j++) {
            const intensity = data[i][j];
            ctx.fillStyle = `rgba(255, 255, 0, ${intensity})`; // Yellow with transparency
            const r = radius * ((j + 1) * scalingFactor) / pointsPerSection;
            const x = center.x + r * Math.cos(i * Math.PI / 180);
            const y = center.y + r * Math.sin(i * Math.PI / 180);
            ctx.beginPath();
            ctx.arc(x, y, 2, 0, 2 * Math.PI);
            ctx.fill();
        }
    }
}

// Function to clear the radar plot
function clearRadarPlot() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    // Optionally reset other states or UI elements if needed
}

// Initial draw
drawRadarPlot();
