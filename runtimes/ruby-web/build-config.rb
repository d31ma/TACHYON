MRuby::Build.new do |conf|
  conf.toolchain
  conf.build_dir = File.join(ENV.fetch('TAC_RUBY_BUILD_DIR'), 'host')
  conf.gem core: 'mruby-bin-mrbc'
end

MRuby::CrossBuild.new('tachyon-ruby') do |conf|
  conf.toolchain :emscripten
  conf.build_dir = File.join(ENV.fetch('TAC_RUBY_BUILD_DIR'), 'tachyon-ruby')
  conf.cc.defines += %w[MRB_INT64 MRB_NO_BOXING MRB_UTF8_STRING MRB_CALL_LEVEL_MAX=10000 MRB_STR_LENGTH_MAX=0 MRB_ARY_LENGTH_MAX=0]
  conf.cc.flags << '-Oz'
  conf.gembox 'stdlib'
  conf.gembox 'stdlib-ext'
  conf.gembox 'math'
  conf.gem core: 'mruby-metaprog'
  conf.gem core: 'mruby-method'
  conf.gem core: 'mruby-error'
  conf.gem core: 'mruby-encoding'
  conf.gem core: 'mruby-strftime'
  conf.gem core: 'mruby-compiler'
  conf.gem core: 'mruby-eval'
  conf.gem core: 'mruby-binding'
  conf.gem File.join(__dir__, 'tachyon-web')
end
