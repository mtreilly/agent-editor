import type { PluginV1 } from '../../src/plugins/types'

const plugin: PluginV1 = {
  name: 'hello-world',
  version: '1.0.0',
  kind: 'ui',
  async activate(ctx) {
    return {
      commands: [
        {
          id: 'hello.say',
          title: 'Say Hello',
          run: async () => {
            // eslint-disable-next-line no-alert
            alert('Hello from plugin!')
          },
        },
      ],
    }
  },
}

export default plugin

