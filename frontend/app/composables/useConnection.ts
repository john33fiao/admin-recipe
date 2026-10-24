export type ConnectionState = 'connecting' | 'connected' | 'reconnecting' | 'unavailable'

export const useConnection = () => useState<ConnectionState>('connection', () => 'connecting')

export function retryConnection() {
  window.dispatchEvent(new Event('app:reconnect'))
}
