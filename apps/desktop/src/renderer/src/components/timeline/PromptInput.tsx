import { useState } from 'react'

interface PromptInputProps {
  onSubmit: (prompt: string) => void
  disabled?: boolean
}

export default function PromptInput({ onSubmit, disabled }: PromptInputProps) {
  const [prompt, setPrompt] = useState('')

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (prompt.trim() && !disabled) {
      onSubmit(prompt.trim())
      setPrompt('')
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex gap-2">
      <input
        type="text"
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        placeholder="Ask the agent to do something..."
        disabled={disabled}
        className="flex-1 rounded-md border border-neutral-700 bg-neutral-900 px-4 py-3 text-sm text-white placeholder-neutral-500 outline-none focus:border-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
      />
      <button
        type="submit"
        disabled={!prompt.trim() || disabled}
        className="rounded-md bg-blue-600 px-6 py-3 text-sm font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
      >
        Send
      </button>
    </form>
  )
}
