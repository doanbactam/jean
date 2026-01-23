import { useState, useCallback } from 'react'
import { Loader2, GitBranch, FolderOpen, AlertCircle } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useProjectsStore } from '@/store/projects-store'
import { useInitGitInFolder, useAddProject } from '@/services/projects'
import { getFilename } from '@/lib/path-utils'

export function GitInitModal() {
  const {
    gitInitModalOpen,
    gitInitModalPath,
    closeGitInitModal,
    setAddProjectDialogOpen,
    addProjectParentFolderId,
  } = useProjectsStore()

  const initGit = useInitGitInFolder()
  const addProject = useAddProject()

  const [error, setError] = useState<string | null>(null)

  const folderName = gitInitModalPath ? getFilename(gitInitModalPath) : ''

  const handleOpenChange = useCallback(
    (open: boolean) => {
      if (!open) {
        setError(null)
        closeGitInitModal()
      }
    },
    [closeGitInitModal]
  )

  const handleInitialize = useCallback(async () => {
    if (!gitInitModalPath) return

    setError(null)

    try {
      // Step 1: Initialize git
      await initGit.mutateAsync(gitInitModalPath)

      // Step 2: Add as project (with parent folder if adding into a folder)
      await addProject.mutateAsync({
        path: gitInitModalPath,
        parentId: addProjectParentFolderId ?? undefined,
      })

      // Close both dialogs on success
      closeGitInitModal()
      setAddProjectDialogOpen(false)
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setError(message)
    }
  }, [
    gitInitModalPath,
    initGit,
    addProject,
    addProjectParentFolderId,
    closeGitInitModal,
    setAddProjectDialogOpen,
  ])

  const isPending = initGit.isPending || addProject.isPending

  return (
    <Dialog open={gitInitModalOpen} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <GitBranch className="h-5 w-5" />
            Initialize Git Repository
          </DialogTitle>
          <DialogDescription>
            The selected folder is not a git repository.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Folder path display */}
          <div className="flex items-center gap-3 rounded-lg bg-muted p-3">
            <FolderOpen className="h-5 w-5 shrink-0 text-muted-foreground" />
            <div className="min-w-0">
              <p className="truncate font-medium">{folderName}</p>
              <p className="truncate text-xs text-muted-foreground">
                {gitInitModalPath}
              </p>
            </div>
          </div>

          {/* Description of what will happen */}
          <div className="space-y-2 text-sm text-muted-foreground">
            <p>Jean will initialize git in this folder:</p>
            <ul className="ml-2 list-inside list-disc space-y-1">
              <li>
                Run <code className="rounded bg-muted px-1">git init</code>
              </li>
              <li>Stage all existing files</li>
              <li>Create an initial commit</li>
            </ul>
          </div>

          {/* Error display */}
          {error && (
            <div className="flex items-start gap-2 rounded-lg bg-destructive/10 p-3 text-destructive">
              <AlertCircle className="mt-0.5 h-5 w-5 shrink-0" />
              <div className="text-sm">{error}</div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={isPending}
          >
            Cancel
          </Button>
          <Button onClick={handleInitialize} disabled={isPending}>
            {isPending && <Loader2 className="h-4 w-4 animate-spin" />}
            {initGit.isPending
              ? 'Initializing...'
              : addProject.isPending
                ? 'Adding project...'
                : 'Initialize Git'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export default GitInitModal
