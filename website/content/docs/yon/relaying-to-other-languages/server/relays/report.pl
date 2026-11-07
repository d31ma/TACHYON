use strict;
use warnings;
use JSON::PP;

my $input = do { local $/; <STDIN> };
my $request = decode_json($input || '{}');
my $body = encode_json({ language => 'Perl', route => $request->{route} });
print encode_json({ status => 200, headers => { 'content-type' => ['application/json'] }, body => $body });
