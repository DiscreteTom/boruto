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
      <label>Window Management: </label>
      <input type="text" v-model="pid" placeholder="pid" />
      <button @click="add" :disabled="ws === null">Add</button>
      <button @click="remove" :disabled="ws === null">Remove</button>
      <button @click="removeAll" :disabled="ws === null">Remove All</button>
      <span>Current PIDs: {{ currentPids }}</span>
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
const pid = ref('')
const currentPids = ref<number[]>([])
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
        | { type: 'currentManagedPids'; pids: number[] }

      if (res.type === 'started') {
        started.value = true
      } else if (res.type === 'stopped') {
        started.value = false
      } else if (res.type === 'currentManagedPids') {
        currentPids.value = res.pids
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
  console.log('add', pid.value)
  ws.value?.send(JSON.stringify({ type: 'add', pid: Number(pid.value) }))
}

function remove() {
  console.log('remove', pid.value)
  ws.value?.send(JSON.stringify({ type: 'remove', pid: Number(pid.value) }))
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
</script>
