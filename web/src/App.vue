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
    </div>

    <br />

    <div>
      <div>
        <label>Control: </label>
        <button @click="toggle" :disabled="ws === null">
          {{ started ? 'Disable' : 'Enable' }}
        </button>
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

const addr = ref('')
const pid = ref('')
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
      // TODO
    }
    ws.value.onclose = () => {
      log.value += 'disconnected\n'
      ws.value = null
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

function update() {
  console.log('update', x.value, y.value)
  ws.value?.send(JSON.stringify({ type: 'update', x: Number(x.value), y: Number(y.value) }))
}
</script>
