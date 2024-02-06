<template>
  <main>
    <div>
      <label>Connection Management: </label>
      <input type="text" v-model="addr" placeholder="ws://localhost:9002" />
      <button @click="ws === null ? connect() : disconnect()">
        {{ ws === null ? 'Connect WebSocket' : 'Disconnect' }}
      </button>
    </div>

    <br />

    <div>
      <div>
        <label>Window Management: </label>
        <button @click="capturing = !capturing" :disabled="ws === null">
          {{ capturing ? 'Stop Capture' : 'Capture a Window' }}
        </button>
        <button @click="removeAll" :disabled="ws === null">Remove All</button>
      </div>
      <div>
        <label>Manually Manage HWND: </label>
        <input type="text" v-model="hwnd" placeholder="hwnd" />
        <button @click="add" :disabled="ws === null">Add</button>
        <button @click="remove" :disabled="ws === null">Remove</button>
      </div>
      <span>Current HWNDs: {{ currentHwnds }}</span>
    </div>

    <br />

    <div>
      <div>
        <label>Control: </label>
        <button @click="toggle" :disabled="ws === null">
          {{ started ? 'Disable' : 'Enable' }}
        </button>
        <label>RPS: {{ rps }}</label>
      </div>
      <label>Min X: </label>
      <input type="number" v-model="minX" placeholder="-500" :disabled="ws === null || !started" />
      <label>Max X: </label>
      <input type="number" v-model="maxX" placeholder="500" :disabled="ws === null || !started" />
      <label>X: </label>
      <input
        type="range"
        :min="minX"
        :max="maxX"
        v-model="x"
        @input="update"
        :disabled="ws === null || !started"
      />
    </div>

    <div>
      <label>Min Y: </label>
      <input type="number" v-model="minY" placeholder="-500" :disabled="ws === null || !started" />
      <label>Max Y: </label>
      <input type="number" v-model="maxY" placeholder="500" :disabled="ws === null || !started" />
      <label>Y: </label>
      <input
        type="range"
        :min="minY"
        :max="maxY"
        v-model="y"
        @input="update"
        :disabled="ws === null || !started"
      />
    </div>

    <br />

    <div>
      <div>
        <label>Log:</label>
        <button @click="log = ''">Clear Log</button>
      </div>
      <div></div>
      <pre>{{ log }}</pre>
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const addr = ref('ws://localhost:9002')
const capturing = ref(false)
const hwnd = ref('')
const currentHwnds = ref<number[]>([])
const started = ref(false)
const ws = ref<null | WebSocket>(null)
const x = ref(0)
const minX = ref(-500)
const maxX = ref(500)
const y = ref(0)
const minY = ref(-500)
const maxY = ref(500)
const log = ref('')

function connect() {
  if (ws.value === null) {
    ws.value = new WebSocket(addr.value)
    ws.value.onopen = () => {
      log.value += 'connected\n'
    }
    ws.value.onmessage = (e) => {
      log.value += e.data + '\n'
      const res = JSON.parse(e.data) as
        | { type: 'started' | 'stopped' }
        | { type: 'currentManagedHwnds'; hwnds: number[] }
        | { type: 'refresh'; started: boolean; hwnds: number[] }

      if (res.type === 'started') {
        started.value = true
      } else if (res.type === 'stopped') {
        started.value = false
      } else if (res.type === 'currentManagedHwnds') {
        currentHwnds.value = res.hwnds
      } else if (res.type === 'refresh') {
        started.value = res.started
        currentHwnds.value = res.hwnds
      }
    }
    ws.value.onclose = () => {
      log.value += 'disconnected\n'
      ws.value = null
    }
    ws.value.onerror = () => {
      log.value += 'error\n'
    }
  }
}
function disconnect() {
  if (ws.value !== null) {
    log.value += 'disconnected\n'
    ws.value.close()
  }
}

function toggle() {
  started.value = !started.value
  ws.value?.send(JSON.stringify({ type: started.value ? 'start' : 'stop' }))
}

function add() {
  console.log('add', hwnd.value)
  ws.value?.send(JSON.stringify({ type: 'add', hwnd: Number(hwnd.value.trim()) }))
}

function remove() {
  console.log('remove', hwnd.value)
  ws.value?.send(JSON.stringify({ type: 'remove', hwnd: Number(hwnd.value.trim()) }))
}

function removeAll() {
  console.log('remove all')
  ws.value?.send(JSON.stringify({ type: 'removeAll' }))
}

let count = 0
const rps = ref(0)
function update() {
  console.log('update', x.value, y.value)
  if (ws.value?.bufferedAmount === 0) {
    ws.value?.send(JSON.stringify({ type: 'update', x: Number(x.value), y: Number(y.value) }))
    count++
  }
}

setInterval(() => {
  rps.value = count
  count = 0
}, 1000)

document.addEventListener('keydown', (e) => {
  if (!capturing.value || e.key !== 'c') return

  ws.value?.send(JSON.stringify({ type: 'capture' }))

  // capture one window at a time
  capturing.value = false
})
</script>
