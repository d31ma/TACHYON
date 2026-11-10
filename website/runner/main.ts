import { RunnerProgram } from './RunnerProgram.ts'

process.exit(await RunnerProgram.run(process.env))
